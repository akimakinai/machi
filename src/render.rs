use bevy::{ecs::entity::EntityHashMap, prelude::*};

use crate::chunk::{CHUNK_HEIGHT, CHUNK_SIZE, Chunk, ChunkUpdated};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderedChunks>()
            .add_observer(chunk_updated);
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
    // naive render
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) -> Result<()> {
    let chunk_id = on.event().event_target();
    let chunk = chunks.get(chunk_id)?;

    let mesh_parent = commands
        .spawn(Transform::from_xyz(
            chunk.position.x as f32 * CHUNK_SIZE as f32,
            0.0,
            chunk.position.y as f32 * CHUNK_SIZE as f32,
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

    let cube = meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)));
    let dirt_material = materials.add(Color::srgb(0.5, 0.25, 0.0));
    let grass_material = materials.add(Color::srgb(0.0, 0.5, 0.0));
    let unknown_material = materials.add(Color::srgb(1.0, 0.0, 1.0));

    commands.entity(mesh_parent).with_children(|parent| {
        for x in 0..CHUNK_SIZE as i32 {
            for z in 0..CHUNK_SIZE as i32 {
                for y in 0..CHUNK_HEIGHT as i32 {
                    let block_type = chunk.get_block(IVec3::new(x as i32, y as i32, z as i32));
                    if block_type != 0 {
                        parent.spawn((
                            Mesh3d(cube.clone()),
                            MeshMaterial3d(match block_type {
                                1 => dirt_material.clone(),
                                2 => grass_material.clone(),
                                _ => unknown_material.clone(),
                            }),
                            Transform::from_xyz(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5),
                            Name::new(format!(
                                "Block ({}, {}, {})",
                                chunk.position.x * CHUNK_SIZE as i32 + x,
                                y,
                                chunk.position.y * CHUNK_SIZE as i32 + z,
                            )),
                        ));
                    }
                }
            }
        }
    });

    Ok(())
}
