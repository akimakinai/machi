//! Fly camera controls a la Minecraft creative mode.

use bevy::{
    camera::RenderTarget, input::mouse::AccumulatedMouseMotion, prelude::*, window::PrimaryWindow,
};

pub struct FlyCamPlugin;

impl Plugin for FlyCamPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (flycam_movement, flycam_rotation));
    }
}

#[derive(Component)]
pub struct FlyCam;

fn flycam_movement(
    mut query: Query<&mut Transform, With<FlyCam>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    for mut transform in &mut query {
        let mut movement = Vec3::ZERO;

        if keyboard.pressed(KeyCode::KeyW) {
            movement += transform.forward().as_vec3().with_y(0.0);
        }
        if keyboard.pressed(KeyCode::KeyS) {
            movement -= transform.forward().as_vec3().with_y(0.0);
        }
        if keyboard.pressed(KeyCode::KeyA) {
            movement += transform.left().as_vec3().with_y(0.0);
        }
        if keyboard.pressed(KeyCode::KeyD) {
            movement += transform.right().as_vec3().with_y(0.0);
        }

        movement = movement.normalize_or_zero();

        if keyboard.pressed(KeyCode::Space) {
            movement += Vec3::Y;
        }
        if keyboard.pressed(KeyCode::ShiftLeft) {
            movement -= Vec3::Y;
        }

        let speed = if keyboard.pressed(KeyCode::ControlLeft) {
            20.0
        } else {
            5.0
        };

        transform.translation += movement * speed * time.delta_secs();
    }
}

fn flycam_rotation(
    mut query: Query<(&mut Transform, &Camera), With<FlyCam>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    windows: Query<&Window>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    time: Res<Time>,
) {
    for (mut transform, camera) in &mut query {
        if !camera.is_active {
            continue;
        }

        let RenderTarget::Window(window_ref) = camera.target else {
            continue;
        };
        let Some(window_ref) = window_ref.normalize(primary_window.single().ok()) else {
            continue;
        };
        let Some(window) = windows.get(window_ref.entity()).ok() else {
            continue;
        };

        if !window.focused {
            return;
        }

        if accumulated_mouse_motion.delta == Vec2::ZERO {
            return;
        }

        let yaw =
            Quat::from_rotation_y(-accumulated_mouse_motion.delta.x * 0.5 * time.delta_secs());
        // TODO: limit pitch to pi/2 -- -pi/2
        let pitch =
            Quat::from_rotation_x(-accumulated_mouse_motion.delta.y * 0.5 * time.delta_secs());
        transform.rotation = yaw * transform.rotation;
        transform.rotation = transform.rotation * pitch;
    }
}
