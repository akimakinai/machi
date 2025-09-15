use bevy::{asset::RenderAssetUsages, ecs::entity::EntityHashMap, mesh::Indices, pbr::wireframe::Wireframe, prelude::*};
use mcubes::MarchingCubes;

use crate::chunk::{CHUNK_HEIGHT, CHUNK_SIZE, Chunk, ChunkUnloaded, ChunkUpdated};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderedChunks>()
            .add_observer(chunk_updated)
            .add_observer(chunk_unloaded);
    }
}

#[derive(Resource, Default)]
struct RenderedChunks(EntityHashMap<RenderedChunk>);

struct RenderedChunk {
    pub chunk_entity: Entity,
    pub position: IVec2,
    pub mesh_parent: Entity,
}

fn chunk_updated(
    on: On<ChunkUpdated>,
    mut commands: Commands,
    chunks: Query<&Chunk>,
    mut rendered: ResMut<RenderedChunks>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) -> Result<()> {
    let chunk_id = on.event().event_target();
    let chunk = chunks.get(chunk_id)?;

    debug!("Chunk updated: {:?}", chunk.position);

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
                chunk_entity: chunk_id,
                position: chunk.position,
                mesh_parent,
            },
        );
    }

    // let cube = meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)));
    // let dirt_material = materials.add(Color::srgb(0.5, 0.25, 0.0));
    let mut grass_material = StandardMaterial::from(Color::srgb(0.0, 0.5, 0.0));
    grass_material.perceptual_roughness = 1.0;
    // grass_material.cull_mode = None;
    let grass_material = materials.add(grass_material);
    // let unknown_material = materials.add(Color::srgb(1.0, 0.0, 1.0));

    let mut values = vec![];

    // TODO: sample outmost layer of neighboring chunks
    for z in -1..(CHUNK_SIZE as i32 + 1) {
        for y in -1..(CHUNK_HEIGHT as i32 + 1) {
            for x in -1..(CHUNK_SIZE as i32 + 1) {
                if x < 0 || x >= CHUNK_SIZE as i32 {
                    values.push(0.0);
                    continue;
                }
                if z < 0 || z >= CHUNK_SIZE as i32 {
                    values.push(0.0);
                    continue;
                }
                if y < 0 || y >= CHUNK_HEIGHT as i32 {
                    values.push(0.0);
                    continue;
                }

                let block_type = chunk.get_block(IVec3::new(x, y, z));
                values.push(if block_type != 0 { 1.0 } else { 0.0 });
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
    let mut normals = vec![];
    let mut uvs = vec![];

    for pos in &mcmesh.vertices {
        positions.push(to_arr(pos.posit));
        normals.push(to_arr(pos.normal));
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
    bvmesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    bvmesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    bvmesh.insert_indices(Indices::U32(indices));

    let bvmesh = meshes.add(bvmesh);

    commands.entity(mesh_parent).with_children(|parent| {
        parent.spawn((
            Mesh3d(bvmesh),
            MeshMaterial3d(grass_material.clone()),
            Transform::from_translation(Vec3::splat(-0.5)),
            Wireframe,
            Name::new(format!(
                "Chunk ({}, {})",
                chunk.position.x, chunk.position.y
            )),
        ));
    });

    Ok(())
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
