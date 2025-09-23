use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
}

#[derive(Debug, Clone)]
pub struct ItemStack {
    /// `item_id < 256` represents blocks
    pub item_id: u32,
    pub quantity: u32,
}
