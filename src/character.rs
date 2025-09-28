// Based on https://github.com/Jondolf/avian/blob/cf1d88a2c032a215633a8c32e8e4bb08b16ae790/crates/avian3d/examples/dynamic_character_3d/plugin.rs

use avian3d::prelude::*;
use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*};
use std::f32::consts::PI;

use crate::{PlayerCamera, pause::PausableSystems};

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(add_ground_shape_caster)
            .add_observer(movement)
            .add_systems(
                Update,
                (
                    keyboard_input,
                    gamepad_input,
                    update_grounded,
                    apply_movement_damping,
                )
                    .chain()
                    .in_set(PausableSystems),
            )
            .add_systems(Update, player_camera_control.in_set(PausableSystems));
    }
}

/// Marker for the character to be controlled by the player.
#[derive(Component)]
pub struct Player;

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
#[derive(Component)]
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

fn add_ground_shape_caster(
    on: On<Add, CharacterController>,
    mut commands: Commands,
    colliders: Query<&Collider>,
) {
    let collider = colliders.get(on.entity).unwrap();
    let mut caster_shape = collider.clone();
    // Create shape caster as a slightly smaller version of collider
    caster_shape.set_scale(Vec3::ONE * 0.99, 10);
    commands.entity(on.entity).insert(
        ShapeCaster::new(caster_shape, Vec3::ZERO, Quat::default(), Dir3::NEG_Y)
            .with_max_distance(0.2),
    );
}

/// A marker component indicating that an entity is on the ground.
#[derive(Component, Default)]
pub struct Grounded(Option<Vec3>);

impl Grounded {
    fn is_grounded(&self) -> bool {
        self.0.is_some()
    }
}

/// Sends [`MovementAction`] events based on keyboard input.
fn keyboard_input(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    players: Query<Entity, With<Player>>,
) {
    let up = keyboard_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let down = keyboard_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);

    let horizontal = right as i8 - left as i8;
    let vertical = up as i8 - down as i8;
    let direction = Vec2::new(horizontal as f32, vertical as f32).clamp_length_max(1.0);

    if direction != Vec2::ZERO {
        for entity in players.iter() {
            commands.trigger(MovementEvent {
                entity,
                kind: MovementEventKind::Move(direction),
            });
        }
    }

    if keyboard_input.just_pressed(KeyCode::Space) {
        for entity in players.iter() {
            commands.trigger(MovementEvent {
                entity,
                kind: MovementEventKind::Jump,
            });
        }
    }
}

/// Sends [`MovementAction`] events based on gamepad input.
fn gamepad_input(
    mut commands: Commands,
    gamepads: Query<&Gamepad>,
    players: Query<Entity, With<Player>>,
) {
    for gamepad in gamepads.iter() {
        if let (Some(x), Some(y)) = (
            gamepad.get(GamepadAxis::LeftStickX),
            gamepad.get(GamepadAxis::LeftStickY),
        ) {
            let direction = Vec2::new(x, y).clamp_length_max(1.0);
            for entity in players.iter() {
                commands.trigger(MovementEvent {
                    entity,
                    kind: MovementEventKind::Move(direction),
                });
            }
        }

        if gamepad.just_pressed(GamepadButton::South) {
            for entity in players.iter() {
                commands.trigger(MovementEvent {
                    entity,
                    kind: MovementEventKind::Jump,
                });
            }
        }
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

fn player_camera_control(
    mouse: Res<AccumulatedMouseMotion>,
    mut player_camera: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
    mut players: Query<&mut Transform, (With<Player>, Without<PlayerCamera>)>,
) -> Result<()> {
    const MOUSE_SENSITIVITY: f32 = 0.01;

    // Apply pitch to camera
    let mut camera_transform = player_camera.single_mut()?;
    let mut rotation = camera_transform.rotation.to_euler(EulerRot::YXZ);
    // TODO: should be divided by resolution
    rotation.1 = (rotation.1 + -mouse.delta.y * MOUSE_SENSITIVITY).clamp(
        -std::f32::consts::FRAC_PI_2 + 0.01,
        std::f32::consts::FRAC_PI_2 - 0.01,
    );
    camera_transform.rotation = Quat::from_euler(EulerRot::YXZ, rotation.0, rotation.1, 0.0);

    // Apply yaw to player, which the character controller uses for movement direction
    let mut player_transform = players.single_mut()?;
    let mut rotation = player_transform.rotation.to_euler(EulerRot::YXZ);
    rotation.0 += -mouse.delta.x * MOUSE_SENSITIVITY;
    player_transform.rotation = Quat::from_euler(EulerRot::YXZ, rotation.0, 0.0, 0.0);

    Ok(())
}
