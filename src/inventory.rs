use bevy::prelude::*;

const MAX_QUANTITY: u32 = 64;

#[derive(Component, Debug, Clone)]
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
}

impl Inventory {
    pub fn add_item_stack(&mut self, item_stack: ItemStack) -> Result<(), ItemStack> {
        let mut remaining = item_stack.quantity;
        for slot in self.slots.iter_mut() {
            if let Some(existing_stack) = slot
                && existing_stack.item_id == item_stack.item_id
            {
                let can_add = MAX_QUANTITY - existing_stack.quantity;
                let to_add = remaining.min(can_add);
                existing_stack.quantity += to_add;
                remaining -= to_add;
                if remaining == 0 {
                    return Ok(());
                }
            }
        }
        for slot in self.slots.iter_mut() {
            if slot.is_none() {
                let to_add = remaining.min(MAX_QUANTITY);
                *slot = Some(ItemStack {
                    item_id: item_stack.item_id,
                    quantity: to_add,
                });
                remaining -= to_add;
                if remaining == 0 {
                    return Ok(());
                }
            }
        }
        if remaining > 0 {
            Err(ItemStack {
                item_id: item_stack.item_id,
                quantity: remaining,
            })
        } else {
            Ok(())
        }
    }
}

pub type ItemId = u32;

#[derive(Debug, Clone)]
pub struct ItemStack {
    /// `item_id < 256` represents blocks
    pub item_id: ItemId,
    pub quantity: u32,
}
