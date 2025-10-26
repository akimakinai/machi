use bevy::prelude::*;

use crate::item::{Item, ItemId, ItemRegistry};

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, register_items);
}

pub struct BoneItem;

impl Item for BoneItem {
    const USABLE: bool = false;
}

fn register_items(mut registry: ResMut<ItemRegistry>, asset_server: Res<AssetServer>) {
    registry.register_item::<BoneItem>(ItemId(257), asset_server.load("textures/items/bone.png"));
}
