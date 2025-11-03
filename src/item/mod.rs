pub mod defs;
pub mod item_core;
pub mod item_icon;
pub mod item_use;

use bevy::prelude::*;

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            item_core::ItemCorePlugin,
            defs::bone::plugin,
            defs::dynamite::plugin,
        ));
    }
}
