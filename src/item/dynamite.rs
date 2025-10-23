use bevy::prelude::*;

use crate::item::{ItemId, ItemRegistry, ItemUse};

pub fn plugin(app: &mut App) {
    app.add_observer(on_use_dynamite)
        .add_systems(Startup, register_items);
}

#[derive(Component)]
pub struct Dynamite;

fn register_items(mut registry: ResMut<ItemRegistry>) {
    registry.register_use::<Dynamite>(ItemId(256));
}

fn on_use_dynamite(on: On<ItemUse<Dynamite>>) {
    info!("Dynamite used by {}", on.event().player());
}
