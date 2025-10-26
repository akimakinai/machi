use bevy::prelude::*;

use crate::{
    item::{Item, ItemId, ItemRegistry},
    ui::item_icon::{ItemIconMaterial, ItemIconRegistry},
};

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, register_items);
}

pub struct BoneItem;

impl Item for BoneItem {
    const USABLE: bool = false;
}

fn register_items(
    mut registry: ResMut<ItemRegistry>,
    mut icon_registry: ResMut<ItemIconRegistry>,
    mut item_icon_mats: ResMut<Assets<ItemIconMaterial>>,
    asset_server: Res<AssetServer>,
) {
    registry.register_item::<BoneItem>(ItemId(257));
    icon_registry.register_item(
        ItemId(257),
        item_icon_mats.add(ItemIconMaterial {
            icon: asset_server.load("textures/items/bone.png"),
        }),
    );
}
