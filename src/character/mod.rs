use bevy::prelude::*;

mod ai;
pub mod controller;
pub mod enemy;
pub mod player;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(player::PlayerPlugin)
            .add_plugins(enemy::EnemyPlugin)
            .add_plugins(ai::AiPlugin)
            .add_plugins(controller::plugin);
    }
}
