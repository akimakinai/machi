use avian3d::prelude::*;

#[derive(PhysicsLayer, Debug, Default)]
pub enum GameLayer {
    #[default]
    Default,
    Terrain,
    Character,
    Object,
    Projectile,
}
