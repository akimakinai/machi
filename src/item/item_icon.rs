use bevy::prelude::*;

#[derive(Component, Clone)]
pub struct ItemIcon(pub Handle<Image>);
