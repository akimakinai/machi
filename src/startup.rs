use bevy::prelude::*;

pub struct StartupPlugin;

impl Plugin for StartupPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Startup,
            (
                StartupSystems::RegisterItems,
                StartupSystems::FinalizeRegisterItems,
                StartupSystems::PostRegisterItems,
                StartupSystems::DevSetup,
            )
                .chain(),
        );
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub enum StartupSystems {
    RegisterItems,
    FinalizeRegisterItems,
    PostRegisterItems,
    DevSetup,
}
