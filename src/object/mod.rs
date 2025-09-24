pub mod item_stack;

use bevy::prelude::*;

/// Manages objects (e.g., item stacks) in the world
pub struct ObjectPlugin;

impl Plugin for ObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(item_stack::ItemStackObjPlugin);
    }
}
