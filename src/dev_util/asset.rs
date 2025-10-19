use std::marker::PhantomData;

use bevy::{asset::AssetTrackingSystems, prelude::*};

pub struct AssetLoadObserverPlugin<A: Asset>(PhantomData<A>);

impl<A: Asset> Default for AssetLoadObserverPlugin<A> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<A: Asset> Plugin for AssetLoadObserverPlugin<A> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            asset_on_load_system::<A>.after(AssetTrackingSystems),
        );
    }
}

/// Fires [`AssetLoad`] event when the asset is loaded.
#[derive(Component)]
pub struct AssetLoadObserved<T: Asset>(pub Handle<T>);

#[derive(EntityEvent)]
pub struct AssetLoad<T: Asset> {
    pub entity: Entity,
    pub handle: Handle<T>,
}

fn asset_on_load_system<A: Asset>(
    mut commands: Commands,
    mut query: Query<(Entity, &AssetLoadObserved<A>)>,
    server: Res<AssetServer>,
) {
    for (entity, asset_handler) in query.iter_mut() {
        if server.load_state(&asset_handler.0).is_loaded() {
            commands
                .entity(entity)
                .trigger(|entity| AssetLoad {
                    entity,
                    handle: asset_handler.0.clone(),
                })
                .remove::<AssetLoadObserved<A>>();
        }
    }
}
