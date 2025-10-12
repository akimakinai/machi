use bevy::{camera::primitives::Aabb, ecs::query::QueryData, prelude::*};

use crate::dev_util::debug_annotation::DebugAnnotUi;

pub(crate) struct TargetPlugin;

impl Plugin for TargetPlugin {
    fn build(&self, app: &mut App) {
        register_annot_target::<AnnotTargetAabb>(app);
    }
}

pub fn register_annot_target<T: AnnotTarget>(app: &mut App) {
    app.add_systems(
        PostUpdate,
        update_annot_target::<T>.in_set(AnnotUpdateSystems),
    );
}

fn update_annot_target<T: AnnotTarget>(
    mut annot_ui: Query<(&DebugAnnotUi, &mut AnnotTargetRect, &Visibility, &Node)>,
    source_query: Query<(&T, T::Source)>,
    camera_query: Query<(&Camera, &GlobalTransform), With<AnnotTargetCamera>>,
) -> Result<()> {
    let Ok(camera) = camera_query.single() else {
        error_once!("No camera with AnnotTargetCamera found.");
        return Ok(());
    };

    for (&DebugAnnotUi(target_id), mut rect, vis, node) in &mut annot_ui {
        if vis == Visibility::Hidden || node.display == Display::None {
            continue;
        }

        let Some((target, source)) = source_query.get(target_id).ok() else {
            error!("Could not get source for annot target: {target_id:?}");
            continue;
        };

        if let Ok(new_rect) = target.annot_target(source, camera) {
            rect.0 = Some(new_rect);
        }
    }

    Ok(())
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub struct AnnotUpdateSystems;

#[derive(Component, Clone, Copy)]
pub struct AnnotTargetCamera;

/// Make [`DebugAnnotationUi`] target the AABB of the entity.
#[derive(Component, Clone, Copy)]
pub struct AnnotTargetAabb;

/// Target rect in logical viewport coordinates.
/// You can update this manually or add a [`AnnotationTarget`] component to update it automatically.
#[derive(Component, Clone, Copy, Default, Debug)]
pub struct AnnotTargetRect(pub Option<Rect>);

// #[cfg(or(feature = "avian3d", feature = "avian2d")))]
// #[derive(Component)]
// struct AnnotationColliderAabb;

pub trait AnnotTarget: Component {
    type Source: QueryData + 'static;

    /// Returns the target rect in logical viewport coordinates.
    fn annot_target(
        &self,
        source: <<Self::Source as QueryData>::ReadOnly as QueryData>::Item<'_, '_>,
        camera: (&Camera, &GlobalTransform),
    ) -> Result<Rect>;
}

impl AnnotTarget for AnnotTargetAabb {
    type Source = (&'static Aabb, &'static GlobalTransform);

    fn annot_target(
        &self,
        (aabb, transform): (&Aabb, &GlobalTransform),
        camera: (&Camera, &GlobalTransform),
    ) -> Result<Rect> {
        let center = camera
            .0
            .world_to_viewport(camera.1, transform.transform_point(aabb.center.to_vec3()))?;

        let mut half_size = Vec2::ZERO;
        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                for z in [-1.0, 1.0] {
                    let corner = transform.transform_point(
                        aabb.center.to_vec3()
                            + Vec3::new(
                                aabb.half_extents.x * x,
                                aabb.half_extents.y * y,
                                aabb.half_extents.z * z,
                            ),
                    );
                    let viewport_pos = camera.0.world_to_viewport(camera.1, corner)?;
                    half_size = half_size.max((viewport_pos - center).abs());
                }
            }
        }

        Ok(Rect::from_center_half_size(center, half_size))
    }
}
