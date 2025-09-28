use bevy::prelude::*;

pub mod block_icon;
pub mod inventory;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((inventory::InventoryUiPlugin, block_icon::BlockIconPlugin));
    }
}
