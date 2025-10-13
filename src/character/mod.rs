// Based on https://github.com/Jondolf/avian/blob/cf1d88a2c032a215633a8c32e8e4bb08b16ae790/crates/avian3d/examples/dynamic_character_3d/plugin.rs

use avian3d::prelude::*;
use bevy::prelude::*;
use std::f32::consts::PI;

use crate::pause::PausableSystems;

mod ai;
pub mod enemy;
pub mod player;

use player::Player;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(player::PlayerPlugin)
            .add_plugins(enemy::EnemyPlugin)
            .add_plugins(ai::AiPlugin)
            .add_observer(movement)
            .add_systems(
                Update,
                (update_grounded, apply_movement_damping)
                    .chain()
                    .in_set(PausableSystems),
            )
            .add_systems(Update, update_ground_shape_caster);
    }
}

/// A movement event
#[derive(EntityEvent)]
pub struct MovementEvent {
    pub entity: Entity,
    pub kind: MovementEventKind,
}

#[derive(Clone, Copy)]
pub enum MovementEventKind {
    Move(Vec2),
    Jump,
}

/// A marker component indicating that an entity is using a character controller.
#[derive(Component, Clone)]
#[require(RigidBody, Collider, LockedAxes::ROTATION_LOCKED, Grounded)]
pub struct CharacterController {
    pub movement_acceleration: f32,
    pub movement_damping_factor: f32,
    pub jump_impulse: f32,
    pub max_slope_angle: Option<f32>,
}

impl Default for CharacterController {
    fn default() -> Self {
        Self {
            movement_acceleration: 60.0,
            movement_damping_factor: 0.9,
            jump_impulse: 7.0,
            max_slope_angle: Some(PI * 0.45),
        }
    }
}

fn update_ground_shape_caster(
    controllers: Query<(Entity, Ref<RigidBodyColliders>), With<CharacterController>>,
    mut commands: Commands,
    colliders: Query<&Collider>,
) -> Result<()> {
    for (id, rb_colliders) in &controllers {
        if !rb_colliders.is_changed() {
            return Ok(());
        }

        let mut caster_shape = colliders
            .get(*rb_colliders.collection().first().unwrap())?
            .clone();
        // Create shape caster as a slightly smaller version of collider
        caster_shape.set_scale(Vec3::ONE * 0.99, 10);
        commands.entity(id).insert(
            ShapeCaster::new(caster_shape, Vec3::ZERO, Quat::default(), Dir3::NEG_Y)
                .with_max_distance(0.2),
        );
    }

    Ok(())
}

/// A marker component indicating that an entity is on the ground.
#[derive(Component, Default)]
pub struct Grounded(Option<Vec3>);

impl Grounded {
    fn is_grounded(&self) -> bool {
        self.0.is_some()
    }
}

/// Updates the [`Grounded`] status for character controllers.
fn update_grounded(
    mut query: Query<
        (&ShapeHits, &Rotation, &CharacterController, &mut Grounded),
        (With<CharacterController>, With<Player>),
    >,
) {
    for (hits, rotation, controller, mut grounded) in &mut query {
        // The character is grounded if the shape caster has a hit with a normal
        // that isn't too steep.
        let ground_normals = hits.iter().filter_map(|hit| {
            let ground_normal: Vec3 = rotation * -hit.normal2;
            if let Some(max_slope_angle) = controller.max_slope_angle {
                if ground_normal.angle_between(Vec3::Y).abs() <= max_slope_angle {
                    return Some(ground_normal);
                }
            } else {
                return Some(ground_normal);
            }
            None
        });

        // Get the steepest ground normal (lowest Y component)
        fn steepest_ground_normal(it: impl Iterator<Item = Vec3>) -> Option<Vec3> {
            let mut min: Option<Vec3> = None;
            for v in it {
                min = Some(match min {
                    Some(m) if v.y < m.y => v,
                    Some(m) => m,
                    None => v,
                });
            }
            min
        }

        if let Some(ground_normal) = steepest_ground_normal(ground_normals) {
            *grounded = Grounded(Some(ground_normal));
        } else {
            *grounded = Grounded(None);
        }
    }
}

/// Responds to [`MovementAction`] events and moves character controllers accordingly.
fn movement(
    on: On<MovementEvent>,
    time: Res<Time>,
    mut controllers: Query<(
        &CharacterController,
        &mut LinearVelocity,
        &Grounded,
        &Transform,
    )>,
) {
    let delta_time = time.delta_secs();

    if let Ok((controller, mut linear_velocity, grounded, transform)) =
        controllers.get_mut(on.event().entity)
    {
        match on.event().kind {
            MovementEventKind::Move(direction) => {
                let mut direction =
                    transform.forward() * direction.y + transform.right() * direction.x;
                if let Some(ground_normal) = grounded.0 {
                    direction = direction - ground_normal * direction.dot(ground_normal);
                }
                linear_velocity.0 += direction * controller.movement_acceleration * delta_time;
            }
            MovementEventKind::Jump => {
                if grounded.is_grounded() {
                    linear_velocity.y = controller.jump_impulse;
                }
            }
        }
    }
}

/// Slows down movement in the XZ plane.
fn apply_movement_damping(mut query: Query<(&CharacterController, &mut LinearVelocity)>) {
    for (controller, mut linear_velocity) in &mut query {
        // We could use `LinearDamping`, but we don't want to dampen movement along the Y axis
        linear_velocity.x *= controller.movement_damping_factor;
        linear_velocity.z *= controller.movement_damping_factor;
    }
}
