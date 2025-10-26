use avian3d::prelude::*;
use bevy::prelude::*;

use crate::{
    explosion::Explode,
    item::{Item, ItemId, ItemRegistry, ItemUse},
    physics::GameLayer,
};

pub fn plugin(app: &mut App) {
    app.add_observer(on_use_dynamite)
        .add_systems(Startup, register_items);
}

pub struct DynamiteItem;

impl Item for DynamiteItem {
    const USABLE: bool = true;
}

fn register_items(mut registry: ResMut<ItemRegistry>, asset_server: Res<AssetServer>) {
    registry.register_item::<DynamiteItem>(
        ItemId(256),
        asset_server.load("textures/items/dynamite.png"),
    );
}

#[derive(Component)]
pub struct ThrownDynamite;

const DYNAMITE_INIT_VEL: f32 = 10.0;
const DYNAMITE_SPAWN_OFFSET: f32 = 0.5;
const DYNAMITE_EXPLOSION_RADIUS: f32 = 8.0;

fn on_use_dynamite(
    on: On<ItemUse<DynamiteItem>>,
    mut commands: Commands,
    transforms: Query<&GlobalTransform>,
    mut meshes: ResMut<Assets<Mesh>>,
) -> Result<()> {
    let user = on.event().user();

    info!("Dynamite used by {}", user);

    let user_tf = *transforms.get(user)?;

    let dir = user_tf
        .forward()
        .with_y(0.5)
        .try_normalize()
        .ok_or_else(|| BevyError::from("Failed to normalize direction"))?;

    let mut dynamite_transform = Transform::from(user_tf);
    dynamite_transform.translation += user_tf.forward() * DYNAMITE_SPAWN_OFFSET;

    let mesh = Cuboid::from_length(0.5);
    commands
        .spawn((
            ThrownDynamite,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d::<StandardMaterial>::default(),
            RigidBody::Dynamic,
            CollisionLayers::new([GameLayer::Projectile], [GameLayer::Terrain]),
            LinearVelocity(dir * DYNAMITE_INIT_VEL),
            mesh.collider(),
            dynamite_transform,
            CollisionEventsEnabled,
        ))
        .observe(on_dynamite_collision);

    Ok(())
}

fn on_dynamite_collision(
    col: On<CollisionStart>,
    transforms: Query<&GlobalTransform>,
    mut commands: Commands,
) -> Result<()> {
    // collider1 is `#[event_target]`
    let dynamite_id = col.event().collider1;
    info!(
        "Dynamite {:?} collided to {:?}, exploding!",
        dynamite_id,
        col.event().collider2,
    );
    commands.entity(dynamite_id).despawn();

    let dynamite_pos = transforms.get(dynamite_id)?;

    commands.write_message(Explode {
        position: dynamite_pos.translation(),
        radius: DYNAMITE_EXPLOSION_RADIUS,
    });

    Ok(())
}
