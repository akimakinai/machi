use bevy::prelude::*;

use crate::{
    character::health::{Health, deal_damage},
    item::ItemStack,
    object::dropped_item::dropped_item_bundle,
    terrain::chunk::WriteBlocks,
};

pub struct ExplosionPlugin;

impl Plugin for ExplosionPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Explode>()
            .add_systems(Startup, setup)
            .add_systems(Update, (break_blocks_on_explode, deal_damage_on_explode))
            .add_systems(Update, update_effects)
            .add_systems(Update, face_camera_billboards)
            .add_systems(
                Update,
                |mut commands: Commands, key: Res<ButtonInput<KeyCode>>| {
                    if key.just_pressed(KeyCode::KeyQ) {
                        commands.write_message(Explode {
                            position: Vec3::new(0.0, 13.0, 0.0),
                            radius: 5.0,
                        });
                    }
                },
            );
    }
}

#[derive(Message, Debug, Clone, Copy)]
pub struct Explode {
    pub position: Vec3,
    pub radius: f32,
}

#[derive(Resource)]
struct ExplosionAssets {
    quad_mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Component)]
struct ExplosionEffect {
    timer: Timer,
    start_scale: f32,
    end_scale: f32,
}

#[derive(Component)]
struct ExplosionBillboard;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut mesh = Mesh::from(Circle::new(1.0));
    // Face -Z
    mesh.invert_winding().unwrap();
    let mesh = meshes.add(mesh);
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(4.0, 2.4, 0.8, 0.7),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        ..default()
    });

    commands.insert_resource(ExplosionAssets {
        quad_mesh: mesh,
        material,
    });
}

fn break_blocks_on_explode(
    mut explode_reader: MessageReader<Explode>,
    mut blocks: WriteBlocks,
    mut commands: Commands,
    assets: Res<ExplosionAssets>,
) -> Result<()> {
    for explode in explode_reader.read() {
        let radius = explode.radius.max(0.1);
        let center = explode.position;

        let min = (center - Vec3::splat(radius + 1.0)).floor().as_ivec3();
        let max = (center + Vec3::splat(radius + 1.0)).ceil().as_ivec3();

        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    let block_pos = IVec3::new(x, y, z);
                    let block_center = block_pos.as_vec3() + Vec3::splat(0.5);
                    let distance = block_center.distance(center);
                    if distance > radius {
                        continue;
                    }

                    let damage = ((radius - distance) / radius).clamp(0.0, 1.0).powf(1.5);
                    if damage <= 0.0 {
                        continue;
                    }

                    if let Some(block) = blocks.damage_block(block_pos, damage)? {
                        commands.spawn((
                            dropped_item_bundle(ItemStack::new(block.as_item_id(), 1)?)?,
                            Transform::from_translation(block_center),
                        ));
                    }
                }
            }
        }

        spawn_explosion_effect(&mut commands, &assets, center, radius);
    }

    Ok(())
}

fn deal_damage_on_explode(
    mut explode_reader: MessageReader<Explode>,
    mut query: Query<(Entity, &GlobalTransform), With<Health>>,
    mut commands: Commands,
) {
    for explode in explode_reader.read() {
        let radius = explode.radius.max(0.1);
        let center = explode.position;
        for (entity, transform) in &mut query {
            let distance = transform.translation().distance(center);
            if distance > radius {
                continue;
            }

            let damage = (1.0 - (distance / radius - 0.5)).clamp(0.0, 1.0).powf(1.5) * 100.0;
            if damage <= 0.0 {
                continue;
            }

            commands.queue(deal_damage(entity, None, damage));
        }
    }
}

fn spawn_explosion_effect(
    commands: &mut Commands,
    assets: &ExplosionAssets,
    position: Vec3,
    radius: f32,
) {
    let start_scale = radius * 0.5;
    let end_scale = radius * 1.1;

    commands.spawn((
        Name::new("Explosion"),
        Mesh3d(assets.quad_mesh.clone()),
        MeshMaterial3d(assets.material.clone()),
        Transform::from_translation(position).with_scale(Vec3::splat(start_scale.max(0.1))),
        Visibility::default(),
        ExplosionEffect {
            timer: Timer::from_seconds(0.3, TimerMode::Once),
            start_scale: start_scale.max(0.1),
            end_scale: end_scale.max(start_scale + 0.1),
        },
        ExplosionBillboard,
    ));
}

fn update_effects(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut ExplosionEffect)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut effect) in &mut query {
        effect.timer.tick(time.delta());
        let t = effect.timer.fraction();
        let scale = effect.start_scale + (effect.end_scale - effect.start_scale) * t;
        transform.scale = Vec3::splat(scale.max(0.01));

        if effect.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn face_camera_billboards(
    camera: Query<&GlobalTransform, With<crate::character::player::PlayerCamera>>,
    mut query: Query<&mut Transform, With<ExplosionBillboard>>,
) -> Result<()> {
    let camera_transform = camera.single()?;
    let camera_position = camera_transform.translation();

    for mut transform in &mut query {
        transform.look_at(camera_position, Vec3::Y);
    }

    Ok(())
}
