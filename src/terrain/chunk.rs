//! Terrain chunks. A chunk is `CHUNK_SIZE * CHUNK_HEIGHT * CHUNK_SIZE` blocks in size.
use bevy::{
    ecs::system::{
        SystemParam,
        lifetimeless::{Read, Write},
    },
    math::bounding::{Aabb3d, RayCast3d},
    platform::collections::HashMap,
    prelude::*,
};

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkMap>()
            .add_observer(update_chunk_map)
            .add_observer(remove_chunk_map);

        app.init_resource::<HoveredBlock>()
            .add_systems(Update, block_hover);

        app.add_systems(Update, chunk_gizmo);
    }
}

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_HEIGHT: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BlockId(pub u8);

impl BlockId {
    pub const AIR: BlockId = BlockId(0);

    /// Blocks rendered as smooth surfaces
    pub const fn is_smooth(self) -> bool {
        self.0 > 0 && self.0 <= 32
    }

    /// Blocks rendered as liquids
    pub const fn is_liquid(self) -> bool {
        self.0 > 32 && self.0 <= 64
    }

    /// Blocks rendered as cubes
    pub const fn is_cube(self) -> bool {
        self.0 > 64
    }
}

#[derive(Component)]
pub struct Chunk {
    pub position: IVec2,
    pub blocks: Box<[[[BlockId; CHUNK_SIZE]; CHUNK_HEIGHT]; CHUNK_SIZE]>,
}

impl Chunk {
    pub fn new(position: IVec2) -> Self {
        Self {
            position,
            blocks: Box::new([[[BlockId(0); CHUNK_SIZE]; CHUNK_HEIGHT]; CHUNK_SIZE]),
        }
    }

    pub fn get_block(&self, position: IVec3) -> BlockId {
        self.blocks[position.x as usize][position.y as usize][position.z as usize]
    }

    pub fn set_block(&mut self, position: IVec3, block: BlockId) {
        self.blocks[position.x as usize][position.y as usize][position.z as usize] = block;
    }
}

#[derive(Resource, Default)]
pub struct ChunkMap(pub HashMap<IVec2, Entity>);

fn update_chunk_map(added: On<Add, Chunk>, mut chunk_map: ResMut<ChunkMap>, chunks: Query<&Chunk>) {
    let entity = added.event().event_target();
    let chunk = chunks.get(entity).unwrap();
    chunk_map.0.insert(chunk.position, entity);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitFace {
    XPos,
    XNeg,
    YPos,
    YNeg,
    ZPos,
    ZNeg,
}

impl HitFace {
    pub fn normal(&self) -> IVec3 {
        match self {
            HitFace::XPos => IVec3::X,
            HitFace::XNeg => -IVec3::X,
            HitFace::YPos => IVec3::Y,
            HitFace::YNeg => -IVec3::Y,
            HitFace::ZPos => IVec3::Z,
            HitFace::ZNeg => -IVec3::Z,
        }
    }
}

impl<'w, 's> BlockRayCast<'w, 's> {
    pub fn ray_cast(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
    ) -> Option<(IVec3, HitFace, Entity)> {
        let mut current_position = origin;
        let step = direction.normalize() * 0.1;
        let mut traveled_distance = 0.0;

        while traveled_distance < max_distance {
            let block_pos = current_position.floor().as_ivec3();
            if let Some((block, entity)) = self.get_block(block_pos)
                && block != BlockId(0)
            {
                let local = (current_position - step) - (block_pos.as_vec3() + Vec3::splat(0.5));
                let dir = Dir3::new(direction).unwrap_or(Dir3::Y);
                let dist = RayCast3d::new(local, dir, 1.0)
                    .aabb_intersection_at(&Aabb3d::new(Vec3::ZERO, Vec3::splat(0.5)))
                    .unwrap_or_default();

                let hit_position = local + dir.as_vec3() * dist;

                let hit_face = if (hit_position.x - 0.5).abs() < f32::EPSILON {
                    HitFace::XPos
                } else if (hit_position.x + 0.5).abs() < f32::EPSILON {
                    HitFace::XNeg
                } else if (hit_position.z - 0.5).abs() < f32::EPSILON {
                    HitFace::ZPos
                } else if (hit_position.z + 0.5).abs() < f32::EPSILON {
                    HitFace::ZNeg
                } else if (hit_position.y - 0.5).abs() < f32::EPSILON {
                    HitFace::YPos
                } else {
                    HitFace::YNeg
                };

                return Some((block_pos, hit_face, entity));
            }
            current_position += step;
            traveled_distance += step.length();
        }
        None
    }

    fn get_block(&self, position: IVec3) -> Option<(BlockId, Entity)> {
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
        if local_y < CHUNK_HEIGHT as i32 {
            let block = chunk.get_block(IVec3::new(local_x, local_y, local_z));
            return Some((block, entity));
        } else {
            // Out of height bounds
            return Some((BlockId(0), entity));
        }
    }
}

#[derive(SystemParam)]
pub struct Blocks<'w, 's> {
    chunks: Query<'w, 's, Write<Chunk>>,
    chunk_map: Res<'w, ChunkMap>,
    commands: Commands<'w, 's>,
}

impl<'w, 's> Blocks<'w, 's> {
    pub fn set_block(&mut self, position: IVec3, block: BlockId) -> Result<()> {
        let chunk_x = position.x.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = position.z.div_euclid(CHUNK_SIZE as i32);
        let local_x = position.x.rem_euclid(CHUNK_SIZE as i32);
        let local_y = position.y;
        let local_z = position.z.rem_euclid(CHUNK_SIZE as i32);

        if local_y < 0 {
            return Ok(());
        }

        let chunk_id = *self
            .chunk_map
            .0
            .get(&IVec2::new(chunk_x, chunk_z))
            .ok_or(BevyError::from("Chunk not found"))?;

        let mut chunk = self.chunks.get_mut(chunk_id)?;
        chunk.set_block(IVec3::new(local_x, local_y, local_z), block);

        let mut updated_chunks = vec![chunk_id];

        // update neighboring chunks if on edge
        if local_x == 0 {
            if let Some(&neighbor_id) = self.chunk_map.0.get(&IVec2::new(chunk_x - 1, chunk_z)) {
                updated_chunks.push(neighbor_id);
            }
        } else if local_x == (CHUNK_SIZE - 1) as i32 {
            if let Some(&neighbor_id) = self.chunk_map.0.get(&IVec2::new(chunk_x + 1, chunk_z)) {
                updated_chunks.push(neighbor_id);
            }
        }
        if local_z == 0 {
            if let Some(&neighbor_id) = self.chunk_map.0.get(&IVec2::new(chunk_x, chunk_z - 1)) {
                updated_chunks.push(neighbor_id);
            }
        } else if local_z == (CHUNK_SIZE - 1) as i32 {
            if let Some(&neighbor_id) = self.chunk_map.0.get(&IVec2::new(chunk_x, chunk_z + 1)) {
                updated_chunks.push(neighbor_id);
            }
        }

        self.commands.trigger(ChunkUpdated(updated_chunks));

        Ok(())
    }
}

#[derive(Event)]
pub struct ChunkUpdated(pub Vec<Entity>);

#[derive(EntityEvent)]
pub struct ChunkUnloaded(Entity);

#[derive(Resource, Default, PartialEq)]
pub struct HoveredBlock(pub Option<(IVec3, HitFace)>);

fn block_hover(
    ray_map: Res<bevy::picking::backend::ray::RayMap>,
    block_raycast: BlockRayCast,
    mut gizmos: Gizmos,
    mut hovered: ResMut<HoveredBlock>,
) -> Result<()> {
    let mut new_hovered = None;

    for (_id, ray) in ray_map.iter() {
        if let Some((block_pos, face, _entity)) =
            block_raycast.ray_cast(ray.origin, ray.direction.as_vec3(), 100.0)
        {
            const GIZMO_COLOR: Color = Color::Srgba(bevy::color::palettes::css::YELLOW);
            let coord = block_pos.as_vec3();
            new_hovered = Some((block_pos, face));

            gizmos.axes(Transform::from_translation(coord + Vec3::splat(0.5)), 2.0);

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

    hovered.set_if_neq(HoveredBlock(new_hovered));

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
