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
            .init_resource::<RenderedChunks>()
            .add_observer(chunk_updated)
            .add_observer(chunk_unloaded);
    }
}

#[derive(Resource, Default)]
struct RenderPluginSettings {
    debug: bool,
}

#[derive(Resource, Default)]
struct RenderedChunks(EntityHashMap<RenderedChunk>);

struct RenderedChunk {
    pub position: IVec2,
    pub mesh_parent: Entity,
}

fn chunk_updated(
    on: On<ChunkUpdated>,
    mut commands: Commands,
    chunks: Query<&Chunk>,
    chunk_map: Res<ChunkMap>,
    mut rendered: ResMut<RenderedChunks>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gizmo_assets: ResMut<Assets<GizmoAsset>>,
    settings: Res<RenderPluginSettings>,
) -> Result<()> {
    for chunk_id in on.event().0.iter().copied() {
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

        debug!("Chunk updated: {:?}", chunk.position);
        debug!(
            "Neighbor chunks = {:?}",
            neighbor_chunks
                .iter()
                .map(|c| c.map(|cc| cc.position))
                .collect::<Vec<_>>()
        );

        let mesh_parent = commands
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
            commands.entity(rc.mesh_parent).despawn();
            rc.mesh_parent = mesh_parent;
        } else {
            rendered.0.insert(
                chunk_id,
                RenderedChunk {
                    position: chunk.position,
                    mesh_parent,
                },
            );
        }

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

                    values.push(if block_id.is_smooth() { 1.0 } else { 0.0 });
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

        commands.entity(mesh_parent).with_children(|parent| {
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

fn chunk_unloaded(
    on: On<ChunkUnloaded>,
    mut commands: Commands,
    mut rendered: ResMut<RenderedChunks>,
) {
    let chunk_id = on.event().event_target();
    if let Some(rc) = rendered.0.remove(&chunk_id) {
        commands.entity(rc.mesh_parent).despawn();
        debug!("Chunk unloaded: {:?}", rc.position);
    }
}
