use bevy::prelude::*;

pub mod log_window;
mod scrollbar;

pub struct DevUtilPlugin;

impl Plugin for DevUtilPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins((log_window::LogWindowPlugin, scrollbar::ScrollbarPlugin));
    }
}
