use bevy::prelude::*;

pub mod log_window;

pub struct DevUtilPlugin;

impl Plugin for DevUtilPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(log_window::LogWindowPlugin);
    }
}
