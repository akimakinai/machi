use bevy::{
    asset::RenderAssetUsages,
    color::palettes::css::{PURPLE, YELLOW},
    ecs::entity::EntityHashMap,
    mesh::{Indices, VertexAttributeValues},
    platform::collections::HashMap,
    prelude::*,
};
use mcubes::MarchingCubes;

use crate::terrain::chunk::BlockId;

use super::chunk::{CHUNK_HEIGHT, CHUNK_SIZE, Chunk, ChunkMap, ChunkUnloaded, ChunkUpdated};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderPluginSettings>()
            .init_resource::<RenderChunkMap>()
            .add_systems(
                Update,
                (create_render_chunk, (update_terrain, update_solid)).chain(),
            )
            .add_observer(chunk_unloaded);
    }
}

#[derive(Resource, Default)]
struct RenderPluginSettings {
    debug: bool,
}

#[derive(Resource, Default)]
struct RenderChunkMap(EntityHashMap<RenderChunk>);

struct RenderChunk {
    pub position: IVec2,
    pub id: Entity,
}

fn create_render_chunk(
    mut reader: MessageReader<ChunkUpdated>,
    chunks: Query<&Chunk>,
    mut commands: Commands,
    mut rendered: ResMut<RenderChunkMap>,
) {
    for &ChunkUpdated(chunk_id) in reader.read() {
        let chunk = chunks.get(chunk_id).unwrap();
        debug!("Chunk updated: {:?}", chunk.position);

        let render_chunk = commands
            .spawn((
                Transform::from_xyz(
                    chunk.position.x as f32 * CHUNK_SIZE as f32,
                    0.0,
                    chunk.position.y as f32 * CHUNK_SIZE as f32,
                ),
                Visibility::Visible,
            ))
            .id();

        if let Some(rc) = rendered.0.get_mut(&chunk_id) {
            commands.entity(rc.id).despawn();
            rc.id = render_chunk;
        } else {
            rendered.0.insert(
                chunk_id,
                RenderChunk {
                    position: chunk.position,
                    id: render_chunk,
                },
            );
        }
    }
}

fn update_terrain(
    mut reader: MessageReader<ChunkUpdated>,
    mut commands: Commands,
    chunks: Query<&Chunk>,
    chunk_map: Res<ChunkMap>,
    rendered: Res<RenderChunkMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gizmo_assets: ResMut<Assets<GizmoAsset>>,
    settings: Res<RenderPluginSettings>,
) -> Result<()> {
    for &ChunkUpdated(chunk_id) in reader.read() {
        let chunk = chunks.get(chunk_id)?;

        let mut neighbor_chunks = vec![];
        for dz in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dz == 0 {
                    neighbor_chunks.push(Some(chunk));
                    continue;
                }
                let neighbor_pos = chunk.position + IVec2::new(dx, dz);
                if let Some(&neighbor_entity) = chunk_map.0.get(&neighbor_pos) {
                    if let Ok(neighbor_chunk) = chunks.get(neighbor_entity) {
                        neighbor_chunks.push(Some(neighbor_chunk));
                        continue;
                    }
                }
                neighbor_chunks.push(None);
            }
        }

        let dxdz_to_idx = |dx: i32, dz: i32| -> usize { ((dz + 1) * 3 + (dx + 1)) as usize };

        debug!(
            "Neighbor chunks = {:?}",
            neighbor_chunks
                .iter()
                .map(|c| c.map(|cc| cc.position))
                .collect::<Vec<_>>()
        );

        let mut base_material = StandardMaterial::from(Color::srgb(1.0, 1.0, 1.0));
        base_material.perceptual_roughness = 1.0;
        let base_material = materials.add(base_material);

        let mut values = vec![];

        let mut block_ids = vec![];

        // TODO: sample outmost layer of neighboring chunks
        for z in -1..(CHUNK_SIZE as i32 + 1) {
            for y in -1..(CHUNK_HEIGHT as i32 + 1) {
                for x in -1..(CHUNK_SIZE as i32 + 1) {
                    let mut dxdz = IVec2::ZERO;
                    if x < 0 {
                        dxdz.x = -1;
                    } else if x >= CHUNK_SIZE as i32 {
                        dxdz.x = 1;
                    }
                    if z < 0 {
                        dxdz.y = -1;
                    } else if z >= CHUNK_SIZE as i32 {
                        dxdz.y = 1;
                    }

                    let block_id = if y < 0 || y >= CHUNK_HEIGHT as i32 {
                        BlockId::AIR
                    } else {
                        let neighbor_idx = dxdz_to_idx(dxdz.x, dxdz.y);
                        if let Some(neighbor_chunk) = neighbor_chunks[neighbor_idx] {
                            let nx = x - dxdz.x * CHUNK_SIZE as i32;
                            let nz = z - dxdz.y * CHUNK_SIZE as i32;
                            neighbor_chunk.get_block(IVec3::new(nx, y, nz))
                        } else {
                            BlockId::AIR
                        }
                    };

                    values.push(if block_id.is_terrain() { 1.0 } else { 0.0 });
                    block_ids.push(block_id);
                }
            }
        }

        let mcmesh = MarchingCubes::new(
            (CHUNK_SIZE + 2, CHUNK_HEIGHT + 2, CHUNK_SIZE + 2),
            (1.0, 1.0, 1.0),
            (1.0, 1.0, 1.0),
            default(),
            values,
            0.5,
        )
        .unwrap()
        .generate(mcubes::MeshSide::OutsideOnly);

        let mut bvmesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );

        let to_arr = |v: lin_alg::f32::Vec3| [v.x, v.y, v.z];

        let mut positions = vec![];
        let mut uvs = vec![];

        for pos in &mcmesh.vertices {
            let position = to_arr(pos.posit);
            positions.push(position);
            uvs.push([0.0, 0.0]);
        }

        let indices = mcmesh
            .indices
            .iter()
            .map(|&i| i as u32)
            // probably CW/CCW reversed
            .rev()
            .collect::<Vec<_>>();

        bvmesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        bvmesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        bvmesh.insert_indices(Indices::U32(indices.clone()));

        // mcubes generates incorrect(?) normals for diagonal parts, so recompute them.
        // Deduplicate vertices to get smooth normals
        deduplicate_vertices(&mut bvmesh);
        bvmesh.compute_normals();

        let bv_normal = bvmesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .unwrap()
            .as_float3()
            .unwrap();
        let bv_position = bvmesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();

        let mut colors = vec![[1.0, 0.0, 1.0, 1.0]; bv_position.len()];

        let mut gizmo = GizmoAsset::default();

        for index in bvmesh.indices().unwrap().iter() {
            let normal = Vec3::from(bv_normal[index]);
            let vert_position = Vec3::from(bv_position[index]);
            let position = (vert_position - normal * 0.1).round();
            let idx = (position.x as usize)
                + (position.y as usize) * (CHUNK_SIZE + 2)
                + (position.z as usize) * (CHUNK_SIZE + 2) * (CHUNK_HEIGHT + 2);
            let block_id = block_ids[idx];

            let color = match block_id {
                BlockId(1) => [0.0, 0.5, 0.0, 1.0],
                BlockId(2) => [0.3, 0.3, 0.3, 1.0],
                _ => [1.0, 0.0, 1.0, 1.0],
            };
            colors[index] = color;

            if !settings.debug {
                continue;
            }
            // Normal
            gizmo.line(
                vert_position,
                vert_position + normal * 0.2,
                match color {
                    [1.0, 0.0, 1.0, 1.0] => PURPLE,
                    _ => YELLOW,
                },
            );
            // Block reference
            gizmo.arrow(
                vert_position,
                position,
                match color {
                    [1.0, 0.0, 1.0, 1.0] => PURPLE,
                    _ => YELLOW,
                },
            );
        }

        bvmesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

        let bvmesh = meshes.add(bvmesh);

        let render_chunk = rendered.0.get(&chunk_id).unwrap().id;

        commands.entity(render_chunk).with_children(|parent| {
            let mut mesh_entity = parent.spawn((
                Mesh3d(bvmesh),
                MeshMaterial3d(base_material.clone()),
                Transform::from_translation(Vec3::splat(-0.5)),
                Name::new(format!(
                    "Chunk ({}, {})",
                    chunk.position.x, chunk.position.y
                )),
            ));
            if settings.debug {
                mesh_entity.with_child(Gizmo {
                    handle: gizmo_assets.add(gizmo),
                    ..default()
                });
            }
        });
    }

    Ok(())
}

/// Deduplicate vertices in the mesh. Normals are not considered.
/// Vertex positions are compared using their bit representation.
fn deduplicate_vertices(mesh: &mut Mesh) {
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();
    let VertexAttributeValues::Float32x2(uvs) = mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap()
    else {
        unreachable!()
    };
    let Indices::U32(indices) = mesh.indices().unwrap() else {
        panic!("Use U32 indices")
    };

    let mut new_positions = vec![];
    let mut new_uvs = vec![];
    let mut new_indices = Vec::with_capacity(indices.len());

    let mut vertex_map = HashMap::new();

    for &i in indices {
        let pos = positions[i as usize];
        let uv = uvs[i as usize];

        let key = (pos[0].to_bits(), pos[1].to_bits(), pos[2].to_bits());
        let new_index = vertex_map.entry(key).or_insert_with(|| {
            let idx = new_positions.len() as u32;
            new_positions.push(pos);
            new_uvs.push(uv);
            idx
        });
        new_indices.push(*new_index);
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, new_positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, new_uvs);
    mesh.insert_indices(Indices::U32(new_indices));
}

fn update_solid(
    mut reader: MessageReader<ChunkUpdated>,
    chunks: Query<&Chunk>,
    render_chunks: Res<RenderChunkMap>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cube_mesh: Local<Handle<Mesh>>,
    // TODO
    mut cube_material: Local<Handle<StandardMaterial>>,
) {
    if *cube_mesh == Handle::default() {
        let cube = Mesh::from(Cuboid::from_length(1.0));
        *cube_mesh = meshes.add(cube);
    }
    if *cube_material == Handle::default() {
        let mut material = StandardMaterial::from(Color::srgb(1.0, 0.0, 1.0));
        material.perceptual_roughness = 1.0;
        *cube_material = materials.add(material);
    }

    for &ChunkUpdated(chunk_id) in reader.read() {
        let chunk = chunks.get(chunk_id).unwrap();

        let render_chunk = render_chunks.0.get(&chunk_id).unwrap().id;

        for z in 0..CHUNK_SIZE as i32 {
            for y in 0..CHUNK_HEIGHT as i32 {
                for x in 0..CHUNK_SIZE as i32 {
                    let block_id = chunk.get_block(IVec3::new(x, y, z));
                    if !block_id.is_solid() {
                        continue;
                    }

                    commands.entity(render_chunk).with_child((
                        Mesh3d(cube_mesh.clone()),
                        MeshMaterial3d(cube_material.clone()),
                        Transform::from_translation(Vec3::new(
                            x as f32 + 0.5,
                            y as f32 + 0.5,
                            z as f32 + 0.5,
                        )),
                        Name::new(format!("Solid Block ({}, {}, {})", x, y, z)),
                    ));
                }
            }
        }
    }
}

fn chunk_unloaded(
    on: On<ChunkUnloaded>,
    mut commands: Commands,
    mut rendered: ResMut<RenderChunkMap>,
) {
    let chunk_id = on.event().event_target();
    if let Some(rc) = rendered.0.remove(&chunk_id) {
        commands.entity(rc.id).despawn();
        debug!("Chunk unloaded: {:?}", rc.position);
    }
}
