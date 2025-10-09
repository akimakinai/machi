use bevy::{camera::primitives::Aabb, prelude::*};

pub(crate) struct TargetPlugin;

impl Plugin for TargetPlugin {
    fn build(&self, app: &mut App) {
        register_callout_target::<CalloutTargetAabb>(app);
    }
}

pub fn register_callout_target<T: CalloutTarget>(app: &mut App) {
    app.add_systems(
        Update,
        update_callout_target::<T>.in_set(CalloutUpdateSystems),
    );
}

fn update_callout_target<T: CalloutTarget>(
    mut query: Query<(&T, &T::Source, &mut CalloutTargetRect)>,
    camera_query: Query<(&Camera, &GlobalTransform), With<CalloutTargetCamera>>,
) -> Result<()> {
    let camera = camera_query.single()?;

    for (target, source, mut rect) in &mut query {
        if let Ok(new_rect) = target.callout_target(source, camera) {
            rect.0 = new_rect;
        }
    }

    Ok(())
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub struct CalloutUpdateSystems;


#[derive(Component)]
pub struct CalloutTargetCamera;

/// Make [`DebugCalloutUi`] target the AABB of the entity.
#[derive(Component)]
pub struct CalloutTargetAabb;

/// Target rect in logical viewport coordinates.
/// You can update this manually or add a [`CalloutTarget`] component to update it automatically.
#[derive(Component, Clone)]
pub struct CalloutTargetRect(pub Rect);

// #[cfg(or(feature = "avian3d", feature = "avian2d")))]
// #[derive(Component)]
// struct CalloutColliderAabb;

pub trait CalloutTarget: Component {
    type Source: Component + 'static;

    /// Returns the target rect in logical viewport coordinates.
    fn callout_target(
        &self,
        source: &Self::Source,
        camera: (&Camera, &GlobalTransform),
    ) -> Result<Rect>;
}

impl CalloutTarget for CalloutTargetAabb {
    type Source = Aabb;

    fn callout_target(
        &self,
        source: &Self::Source,
        camera: (&Camera, &GlobalTransform),
    ) -> Result<Rect> {
        let min = camera
            .0
            .world_to_viewport(camera.1, source.min().to_vec3())?;
        let max = camera
            .0
            .world_to_viewport(camera.1, source.max().to_vec3())?;

        Ok(Rect::from_corners(min, max))
    }
}
