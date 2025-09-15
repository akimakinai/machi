use bevy::{
    ecs::system::{SystemParam, lifetimeless::Read},
    platform::collections::HashMap,
    prelude::*,
};

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkMap>()
            .add_systems(Update, update_chunk_map)
            .add_observer(remove_chunk_map);

        app.add_observer(update_new_chunks)
            .add_systems(Update, (block_hover_gizmo, chunk_gizmo));
    }
}

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_HEIGHT: usize = 256;

#[derive(Component)]
pub struct Chunk {
    pub position: IVec2,
    pub data: Box<[[[u8; CHUNK_SIZE]; CHUNK_HEIGHT]; CHUNK_SIZE]>,
}

impl Chunk {
    pub fn new(position: IVec2) -> Self {
        Self {
            position,
            data: Box::new([[[0; CHUNK_SIZE]; CHUNK_HEIGHT]; CHUNK_SIZE]),
        }
    }

    pub fn get_block(&self, position: IVec3) -> u8 {
        self.data[position.x as usize][position.y as usize][position.z as usize]
    }

    pub fn set_block(&mut self, position: IVec3, block: u8) {
        self.data[position.x as usize][position.y as usize][position.z as usize] = block;
    }
}

#[derive(Resource, Default)]
pub struct ChunkMap(pub HashMap<IVec2, Entity>);

fn update_chunk_map(
    mut chunk_map: ResMut<ChunkMap>,
    chunks: Query<(Entity, &Chunk), Added<Chunk>>,
) {
    for (entity, chunk) in &chunks {
        chunk_map.0.insert(chunk.position, entity);
    }
}

fn remove_chunk_map(
    removed: On<Remove, Chunk>,
    mut chunk_map: ResMut<ChunkMap>,
    chunks: Query<&Chunk>,
) {
    let chunk_id = removed.event().event_target();
    chunk_map.0.remove(&chunks.get(chunk_id).unwrap().position);
}

#[derive(SystemParam)]
pub struct BlockRayCast<'w, 's> {
    chunks: Query<'w, 's, (Entity, Read<Chunk>)>,
    chunk_map: Res<'w, ChunkMap>,
}

impl<'w, 's> BlockRayCast<'w, 's> {
    pub fn ray_cast(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
    ) -> Option<(IVec3, Entity)> {
        let mut current_position = origin;
        let step = direction.normalize() * 0.1;
        let mut traveled_distance = 0.0;

        while traveled_distance < max_distance {
            let block_pos = current_position.floor().as_ivec3();
            if let Some((block, entity)) = self.get_block(block_pos)
                && block != 0
            {
                return Some((block_pos, entity));
            }
            current_position += step;
            traveled_distance += step.length();
        }
        None
    }

    fn get_block(&self, position: IVec3) -> Option<(u8, Entity)> {
        let chunk_x = position.x.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = position.z.div_euclid(CHUNK_SIZE as i32);
        let local_x = position.x.rem_euclid(CHUNK_SIZE as i32);
        let local_y = position.y;
        let local_z = position.z.rem_euclid(CHUNK_SIZE as i32);

        if local_y < 0 {
            return None;
        }

        let (entity, chunk) = self
            .chunks
            .get(*self.chunk_map.0.get(&IVec2::new(chunk_x, chunk_z))?)
            .ok()?;
        if chunk.position == IVec2::new(chunk_x, chunk_z) {
            if local_y < CHUNK_HEIGHT as i32 {
                let block = chunk.get_block(IVec3::new(local_x, local_y, local_z));
                return Some((block, entity));
            } else {
                // Out of height bounds
                return Some((0, entity));
            }
        }

        // Chunk not found
        None
    }
}

#[derive(EntityEvent)]
pub struct ChunkUpdated(Entity);

fn update_new_chunks(added: On<Add, Chunk>, mut commands: Commands) {
    commands.trigger(ChunkUpdated(added.event().event_target()));
}

#[derive(EntityEvent)]
pub struct ChunkUnloaded(Entity);

// debug systems

fn block_hover_gizmo(
    ray_map: Res<bevy::picking::backend::ray::RayMap>,
    block_raycast: BlockRayCast,
    mut gizmos: Gizmos,
) -> Result<()> {
    for (_id, ray) in ray_map.iter() {
        if let Some((block_pos, _face)) =
            block_raycast.ray_cast(ray.origin, ray.direction.as_vec3(), 100.0)
        {
            const GIZMO_COLOR: Color = Color::Srgba(bevy::color::palettes::css::YELLOW);
            let coord = block_pos.as_vec3();
            gizmos.linestrip(
                [
                    coord + Vec3::new(0.0, 0.0, 0.0),
                    coord + Vec3::new(1.0, 0.0, 0.0),
                    coord + Vec3::new(1.0, 0.0, 1.0),
                    coord + Vec3::new(0.0, 0.0, 1.0),
                    coord + Vec3::new(0.0, 0.0, 0.0),
                    coord + Vec3::new(0.0, 1.0, 0.0),
                    coord + Vec3::new(0.0, 1.0, 0.0),
                    coord + Vec3::new(1.0, 1.0, 0.0),
                    coord + Vec3::new(1.0, 1.0, 1.0),
                    coord + Vec3::new(0.0, 1.0, 1.0),
                    coord + Vec3::new(0.0, 1.0, 0.0),
                ],
                GIZMO_COLOR,
            );
            gizmos.line(
                coord + Vec3::new(1.0, 0.0, 0.0),
                coord + Vec3::new(1.0, 1.0, 0.0),
                GIZMO_COLOR,
            );
            gizmos.line(
                coord + Vec3::new(1.0, 0.0, 1.0),
                coord + Vec3::new(1.0, 1.0, 1.0),
                GIZMO_COLOR,
            );
            gizmos.line(
                coord + Vec3::new(0.0, 0.0, 1.0),
                coord + Vec3::new(0.0, 1.0, 1.0),
                GIZMO_COLOR,
            );
        }
    }
    Ok(())
}

fn chunk_gizmo(chunks: Query<&Chunk>, mut gizmos: Gizmos) {
    for chunk in &chunks {
        const GIZMO_COLOR: Color = Color::Srgba(bevy::color::palettes::css::BLUE);
        let base = chunk.position.as_vec2() * CHUNK_SIZE as f32;
        for pos in [
            Vec2::new(0., 0.),
            Vec2::new(1., 0.),
            Vec2::new(0., 1.),
            Vec2::new(1., 1.),
        ] {
            gizmos.line(
                Vec3::new(
                    base.x + pos.x * CHUNK_SIZE as f32,
                    0.0,
                    base.y + pos.y * CHUNK_SIZE as f32,
                ),
                Vec3::new(
                    base.x + pos.x * CHUNK_SIZE as f32,
                    CHUNK_HEIGHT as f32,
                    base.y + pos.y * CHUNK_SIZE as f32,
                ),
                GIZMO_COLOR,
            );
        }
    }
}
