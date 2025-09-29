use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*};

use crate::{
    character::{MovementEvent, MovementEventKind},
    pause::PausableSystems,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (keyboard_input, gamepad_input).in_set(PausableSystems),
        )
        .add_systems(Update, player_camera_control.in_set(PausableSystems));
    }
}

/// Marker for the character to be controlled by the player.
#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerCamera;

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
