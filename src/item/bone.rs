use bevy::prelude::*;

use crate::{
    item::{ItemKind, item_icon::ItemIcon},
    startup::StartupSystems,
};

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, register_bone.in_set(StartupSystems::RegisterItems));
}

fn register_bone(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        ItemKind::new("bone"),
        ItemIcon(asset_server.load("textures/items/bone.png")),
    ));
}
