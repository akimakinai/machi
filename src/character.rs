use avian3d::prelude::{forces::ForcesItem, *};
use bevy::prelude::*;

use crate::physics::GameLayer;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(character_controller_added)
            .add_systems(FixedUpdate, character_movement);
    }
}

/// Simple floating character controller.
#[derive(Component)]
#[require(Transform, RigidBody::Dynamic, LockedAxes::ROTATION_LOCKED)]
pub struct CharacterController {
    pub speed: f32,
    pub floating_height: f32,
}

#[derive(Component)]
struct CharacterControllerSensor;

fn character_controller_added(on: On<Add, CharacterController>, mut commands: Commands) {
    let id = on.event_target();
    commands.entity(id).with_child((
        CharacterControllerSensor,
        Transform::default(),
        RayCaster::new(Vec3::ZERO, Dir3::NEG_Y).with_query_filter(
            SpatialQueryFilter::from_excluded_entities([id]).with_mask(GameLayer::Terrain),
        ),
    ));
}

fn character_movement(
    mut controllers: Query<(NameOrEntity, &CharacterController, Forces, &Children), With<CharacterController>>,
    mut sensors: Query<&RayHits, With<CharacterControllerSensor>>,
    time: Res<Time<Fixed>>,
) {
    for (name_or_entity, cc, forces, children) in &mut controllers {
        debug!("CharacterController: {name_or_entity}");
        for child in children.iter() {
            if let Ok(ray_hits) = sensors.get_mut(child) {
                debug!("RayHits: {ray_hits:?}");
                character_movement_inner(cc, ray_hits, forces, &time);
                break;
            }
        }
    }
}

fn character_movement_inner(
    cc: &CharacterController,
    ray_hits: &RayHits,
    mut forces: ForcesItem,
    time: &Res<Time<Fixed>>,
) {
    let Some(first_hit) = ray_hits.iter_sorted().next() else {
        return;
    };
    let sub = cc.floating_height - first_hit.distance;
    if sub < 0.0 {
        debug!("Apply force");
        forces.apply_force(Vec3::Y * sub * time.delta_secs());
    }
}
