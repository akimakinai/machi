use bevy::prelude::*;

pub mod hotbar;
pub mod inventory;
pub mod item_icon;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            inventory::InventoryUiPlugin,
            item_icon::plugin,
            hotbar::HotbarPlugin,
        ));
    }
}
