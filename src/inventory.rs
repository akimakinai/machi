use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
}

pub type ItemId = u32;

#[derive(Debug, Clone)]
pub struct ItemStack {
    /// `item_id < 256` represents blocks
    pub item_id: ItemId,
    pub quantity: u32,
}
