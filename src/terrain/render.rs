use avian3d::prelude::*;
use bevy::{
    asset::{RenderAssetUsages, uuid_handle},
    color::palettes::css::{PURPLE, YELLOW},
    ecs::entity::{EntityHashMap, EntityHashSet},
    image::ImageAddressMode,
    mesh::{Indices, VertexAttributeValues},
    pbr::{ExtendedMaterial, MaterialExtension},
    platform::collections::HashMap,
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    tasks::{AsyncComputeTaskPool, Task},
};
use mcubes::MarchingCubes;

use crate::{physics::GameLayer, terrain::chunk::BlockId};

use super::chunk::{CHUNK_HEIGHT, CHUNK_SIZE, Chunk, ChunkMap, ChunkUnloaded, ChunkUpdated};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderPluginSettings>()
            .init_resource::<RenderChunkMap>()
            .add_message::<RenderChunkSpawned>()
            .add_plugins(MaterialPlugin::<ExtendedArrayTextureMaterial>::default())
            .add_systems(Startup, setup_terrain_texture)
            .add_systems(Update, create_array_texture)
            .add_systems(Update, generate_terrain_mesh)
            .add_systems(Update, (spawn_generated_terrain_mesh, update_solid).chain())
            .add_observer(chunk_unloaded);
    }
}

#[derive(Resource, Default, Clone)]
struct RenderPluginSettings {
    /// Enable debug gizmos
    debug: bool,
}

#[derive(Resource, Default)]
struct RenderChunkMap(EntityHashMap<RenderChunk>);

struct RenderChunk {
    pub position: IVec2,
    pub id: Entity,
}

const TERRAIN_SHADER_PATH: &str = "shaders/terrain_texture.wgsl";

#[derive(Resource)]
struct TerrainTexture {
    array_generated: bool,
    array_texture: Handle<Image>,
    array_normal: Handle<Image>,
    material_handle: Handle<ExtendedArrayTextureMaterial>,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct ArrayTextureMaterial {
    #[texture(100, dimension = "2d_array")]
    #[sampler(101)]
    array_texture: Handle<Image>,
    #[texture(102, dimension = "2d_array")]
    #[sampler(103)]
    array_normal: Handle<Image>,
}

impl MaterialExtension for ArrayTextureMaterial {
    fn fragment_shader() -> ShaderRef {
        TERRAIN_SHADER_PATH.into()
    }
}

type ExtendedArrayTextureMaterial = ExtendedMaterial<StandardMaterial, ArrayTextureMaterial>;

fn setup_terrain_texture(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(TerrainTexture {
        array_generated: false,
        array_texture: asset_server.load("textures/array_texture.png"),
        array_normal: asset_server.load("textures/normal_map.png"),
        material_handle: uuid_handle!("1fe9417f-ecee-42dd-a4cc-37af36e7933b"),
    });
}

fn create_array_texture(
    mut terrain_texture: ResMut<TerrainTexture>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ExtendedArrayTextureMaterial>>,
) -> Result<()> {
    if terrain_texture.array_generated {
        return Ok(());
    }
    if !asset_server
        .get_load_state(&terrain_texture.array_texture)
        .is_some_and(|st| st.is_loaded())
    {
        return Ok(());
    }

    if !asset_server
        .get_load_state(&terrain_texture.array_normal)
        .is_some_and(|st| st.is_loaded())
    {
        return Ok(());
    }

    let mut process = |handle: &Handle<Image>| {
        let image = images
            .get_mut(handle)
            .expect("Image should have been loaded");

        // Convert array texture assuming 1:1 aspect ratio
        debug_assert_eq!(image.height() % image.width(), 0);
        let array_layers = image.height() / image.width();
        image.reinterpret_stacked_2d_as_array(array_layers);

        let desc = image.sampler.get_or_init_descriptor();
        desc.address_mode_u = ImageAddressMode::Repeat;
        desc.address_mode_v = ImageAddressMode::Repeat;
    };

    process(&terrain_texture.array_texture);
    process(&terrain_texture.array_normal);

    materials.insert(
        terrain_texture.material_handle.id(),
        ExtendedMaterial {
            base: StandardMaterial::default(),
            extension: ArrayTextureMaterial {
                array_texture: terrain_texture.array_texture.clone(),
                array_normal: terrain_texture.array_normal.clone(),
            },
        },
    )?;

    terrain_texture.array_generated = true;

    debug!("Array texture created");

    Ok(())
}

/// Pending aasynchronous mesh generation task for a terrain chunk.
#[derive(Component)]
struct PendingChunk(Task<PendingChunkResult>);

struct PendingChunkResult {
    mesh: Mesh,
    chunk_id: Entity,
    gizmo: GizmoAsset,
}

fn generate_terrain_mesh(
    mut reader: MessageReader<ChunkUpdated>,
    mut commands: Commands,
    chunks: Query<&Chunk>,
    chunk_map: Res<ChunkMap>,
    settings: Res<RenderPluginSettings>,
    mut dedup: Local<EntityHashSet>,
) -> Result<()> {
    dedup.clear();

    let task_pool = AsyncComputeTaskPool::get();

    for &ChunkUpdated(chunk_id) in reader.read() {
        if !dedup.insert(chunk_id) {
            continue;
        }

        let chunk = chunks.get(chunk_id)?;

        let span = debug_span!("Update terrain", chunk_pos = ?chunk.position).entered();

        let mut neighbor_chunks = vec![];
        for dz in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dz == 0 {
                    neighbor_chunks.push(Some(chunk));
                    continue;
                }
                let neighbor_pos = chunk.position + IVec2::new(dx, dz);
                if let Some(&neighbor_entity) = chunk_map.0.get(&neighbor_pos)
                    && let Ok(neighbor_chunk) = chunks.get(neighbor_entity)
                {
                    neighbor_chunks.push(Some(neighbor_chunk));
                    continue;
                }
                neighbor_chunks.push(None);
            }
        }

        let dxdz_to_idx = |dx: i32, dz: i32| -> usize { ((dz + 1) * 3 + (dx + 1)) as usize };

        let mut values = vec![];

        let mut block_ids = vec![];
        let mut durability_vals = vec![];

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

                    let (block_id, durability) = if y < 0 || y >= CHUNK_HEIGHT as i32 {
                        (BlockId::AIR, 1.0)
                    } else {
                        let neighbor_idx = dxdz_to_idx(dxdz.x, dxdz.y);
                        if let Some(neighbor_chunk) = neighbor_chunks[neighbor_idx] {
                            let nx = x - dxdz.x * CHUNK_SIZE as i32;
                            let nz = z - dxdz.y * CHUNK_SIZE as i32;
                            (
                                neighbor_chunk.get_block(IVec3::new(nx, y, nz)),
                                neighbor_chunk.get_durability(IVec3::new(nx, y, nz)),
                            )
                        } else {
                            (BlockId::AIR, 1.0)
                        }
                    };

                    values.push(if block_id.is_terrain() { 1.0 } else { 0.0 });
                    block_ids.push(block_id);

                    durability_vals.push(durability);
                }
            }
        }

        let settings = settings.clone();
        let task = task_pool.spawn(async move {
            let mc_span = debug_span!("Marching Cubes").entered();
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
            mc_span.exit();

            let bv_span = debug_span!("Bevy Mesh Generation").entered();
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

            bv_span.exit();

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

            let mut colors = vec![[0.0, 0.0, 1.0, 0.0]; bv_position.len()];

            let mut uv1 = vec![[0.0, 0.0]; bv_position.len()];

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
                    BlockId(1) => [1.0, 0.0, 0.0, 0.0],
                    BlockId(2) => [0.0, 0.1, 0.0, 0.0],
                    _ => [1.0, 0.0, 1.0, 1.0],
                };
                colors[index] = color;

                let durability = durability_vals[idx];
                uv1[index] = [durability, 0.0];

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
            bvmesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uv1);

            PendingChunkResult {
                mesh: bvmesh,
                chunk_id,
                gizmo,
            }
        });

        commands.spawn(PendingChunk(task));

        span.exit();
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

/// Message sent when a render chunk is (re)spawned.
#[derive(Message)]
struct RenderChunkSpawned {
    render_chunk: Entity,
    chunk: Entity,
}

fn spawn_generated_terrain_mesh(
    mut commands: Commands,
    mut pending: Query<(Entity, &mut PendingChunk)>,
    chunks: Query<&Chunk>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut gizmo_assets: ResMut<Assets<GizmoAsset>>,
    mut rendered: ResMut<RenderChunkMap>,
    terrain_texture: Res<TerrainTexture>,
    settings: Res<RenderPluginSettings>,
) {
    for (entity, mut pending_chunk) in &mut pending {
        let Some(result) = bevy::tasks::futures::check_ready(&mut pending_chunk.0) else {
            continue;
        };
        commands.entity(entity).despawn();

        let PendingChunkResult {
            mesh,
            chunk_id,
            gizmo,
        } = result;

        let bvmesh = meshes.add(mesh);

        let Ok(chunk) = chunks.get(chunk_id) else {
            continue;
        };

        let render_chunk = commands
            .spawn((
                Name::new(format!(
                    "Render Chunk ({}, {})",
                    chunk.position.x, chunk.position.y
                )),
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

        commands.entity(render_chunk).with_children(|parent| {
            let mut mesh_entity = parent.spawn((
                Mesh3d(bvmesh),
                MeshMaterial3d(terrain_texture.material_handle.clone()),
                RigidBody::Static,
                ColliderConstructor::TrimeshFromMesh,
                CollisionLayers::new(
                    [GameLayer::Terrain],
                    [GameLayer::Default, GameLayer::Character, GameLayer::Object],
                ),
                Transform::from_translation(Vec3::splat(-0.5)),
                Name::new(format!(
                    "Render Chunk Mesh ({}, {})",
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

        commands.write_message(RenderChunkSpawned {
            render_chunk,
            chunk: chunk_id,
        });
    }
}

fn update_solid(
    mut reader: MessageReader<RenderChunkSpawned>,
    chunks: Query<&Chunk>,
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

    for &RenderChunkSpawned {
        render_chunk,
        chunk,
    } in reader.read()
    {
        let chunk = chunks.get(chunk).unwrap();

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
                        RigidBody::Static,
                        ColliderConstructor::ConvexHullFromMesh,
                        CollisionLayers::new(
                            [GameLayer::Terrain],
                            [GameLayer::Default, GameLayer::Character, GameLayer::Object],
                        ),
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
