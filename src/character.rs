use avian3d::prelude::{forces::ForcesItem, *};
use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*};

use crate::physics::GameLayer;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(character_controller_added)
            .add_systems(FixedUpdate, character_movement)
            .add_systems(FixedUpdate, player_control);
    }
}

/// Simple floating character controller.
#[derive(Component)]
#[require(Transform, RigidBody::Dynamic, LockedAxes::ROTATION_LOCKED)]
pub struct CharacterController {
    pub speed: f32,
    pub floating_height: f32,
    pub shape: Collider,
}

#[derive(Component)]
struct CharacterControllerSensor;

fn character_controller_added(
    on: On<Add, CharacterController>,
    mut commands: Commands,
    controllers: Query<&CharacterController>,
) {
    let id = on.event_target();
    let collider = controllers.get(id).unwrap().shape.clone();

    commands.entity(id).with_child((
        CharacterControllerSensor,
        Transform::default(),
        ShapeCaster::new(collider, Vec3::ZERO, Quat::default(), Dir3::NEG_Y).with_query_filter(
            SpatialQueryFilter::from_excluded_entities([id]).with_mask(GameLayer::Terrain),
        ),
    ));
}

fn character_movement(
    mut controllers: Query<(&CharacterController, Forces), With<CharacterController>>,
    sensors: Query<(NameOrEntity, &ShapeHits, &ChildOf), With<CharacterControllerSensor>>,
) {
    for (name, sensor, child_of) in &sensors {
        let Ok((cc, forces)) = controllers.get_mut(child_of.parent()) else {
            error!("Parent of CharacterControllerSensor({name}) is not a CharacterController");
            continue;
        };
        character_movement_inner(cc, sensor, forces);
    }
}

fn character_movement_inner(cc: &CharacterController, hits: &ShapeHits, mut forces: ForcesItem) {
    let force = hits
        .iter()
        .map(|hit| (cc.floating_height - hit.distance) * hit.normal1)
        .sum::<Vec3>();
    if force.length_squared() < 0.01 {
        return;
    }
    forces.apply_force(force);
}

#[derive(Component)]
pub struct Player;

fn player_control(
    mouse: Res<AccumulatedMouseMotion>,
    kb: Res<ButtonInput<KeyCode>>,
    mut controllers: Query<
        (&CharacterController, &mut Transform, &mut LinearVelocity),
        With<Player>,
    >,
    time: Res<Time>,
) {
    for (cc, mut transform, mut velocity) in &mut controllers {
        let mut rotation = transform.rotation.to_euler(EulerRot::YXZ);
        rotation.0 += -mouse.delta.x * 0.5 * time.delta_secs();
        rotation.1 = (rotation.1 + -mouse.delta.y * 0.5 * time.delta_secs()).clamp(
            -std::f32::consts::FRAC_PI_2 + 0.01,
            std::f32::consts::FRAC_PI_2 - 0.01,
        );
        transform.rotation = Quat::from_euler(EulerRot::YXZ, rotation.0, rotation.1, 0.0);

        let mut direction = Vec3::ZERO;
        if kb.pressed(KeyCode::KeyW) {
            direction += transform.forward().as_vec3();
        }
        if kb.pressed(KeyCode::KeyS) {
            direction += transform.back().as_vec3();
        }
        if kb.pressed(KeyCode::KeyA) {
            direction += transform.left().as_vec3();
        }
        if kb.pressed(KeyCode::KeyD) {
            direction += transform.right().as_vec3();
        }
        if direction == Vec3::ZERO {
            // FIXME: setting velocity to exact zero causes panic in Avian
            if velocity.0.xz().length_squared() > 0.01 {
                velocity.0 *= Vec3::new(0.01, 1.0, 0.01);
            }
            continue;
        }
        let direction = direction.normalize();
        velocity.0 = (direction * cc.speed).with_y(velocity.0.y);
    }
}
