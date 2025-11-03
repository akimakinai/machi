use bevy::{platform::collections::HashMap, prelude::*};
use moonshine_kind::Instance;
use std::sync::Arc;

use crate::startup::StartupSystems;

pub struct ItemCorePlugin;

impl Plugin for ItemCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemIndex>().add_systems(
            Startup,
            register_item_kinds.in_set(StartupSystems::FinalizeRegisterItems),
        );
    }
}

pub type ItemId = Instance<ItemKind>;

#[derive(Component, Debug, Clone, Copy)]
pub struct ItemStack {
    /// `item_id < 256` represents blocks
    pub item_id: ItemId,
    /// `0 < quantity <= MAX_QUANTITY`
    quantity: u32,
}

impl ItemStack {
    pub const MAX_QUANTITY: u32 = 64;

    pub fn new(item_id: ItemId, quantity: u32) -> Result<ItemStack> {
        if quantity == 0 || quantity > Self::MAX_QUANTITY {
            return Err("ItemStack quantity must be between 1 and MAX_QUANTITY".into());
        }
        Ok(Self { item_id, quantity })
    }

    pub fn quantity(&self) -> u32 {
        self.quantity
    }

    pub fn decrease_quantity(&mut self, amount: u32) -> Result<()> {
        if amount == 0 {
            return Ok(());
        }
        if amount > self.quantity {
            return Err("Cannot decrease ItemStack quantity by the specified amount".into());
        }
        self.quantity -= amount;
        Ok(())
    }

    pub fn set_quantity(&mut self, quantity: u32) -> Result<()> {
        if quantity == 0 || quantity > Self::MAX_QUANTITY {
            return Err("ItemStack quantity must be between 1 and MAX_QUANTITY".into());
        }
        self.quantity = quantity;
        Ok(())
    }
}

/// Item entity marker trait. Entity with this component represents an item kind.
#[derive(Component, Debug, Clone)]
pub struct ItemKind(pub Arc<str>);

impl ItemKind {
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self(name.into())
    }
}

#[derive(Resource, Default)]
pub struct ItemIndex {
    items: HashMap<Arc<str>, Instance<ItemKind>>,
}

impl ItemIndex {
    pub fn get(&self, name: &str) -> Option<ItemId> {
        self.items.get(name).copied()
    }
}

pub fn register_item_kinds(
    q_item_kind: Query<(Instance<ItemKind>, &ItemKind)>,
    mut item_index: ResMut<ItemIndex>,
) {
    for (item_id, item_kind) in q_item_kind.iter() {
        item_index.items.insert(item_kind.0.clone(), item_id);
    }
}
