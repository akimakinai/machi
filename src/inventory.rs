use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
}

impl Inventory {
    /// Returns `Err(remaining)` if there is not enough space.
    pub fn add_item_stack(&mut self, item_stack: ItemStack) -> Result<(), ItemStack> {
        let mut remaining = item_stack.quantity;
        for slot in self.slots.iter_mut() {
            if let Some(existing_stack) = slot
                && existing_stack.item_id == item_stack.item_id
            {
                let can_add = ItemStack::MAX_QUANTITY - existing_stack.quantity;
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
                // If the slot is empty, we can put all remaining items here
                // since `remaining <= MAX_QUANTITY`.
                *slot = Some(ItemStack {
                    item_id: item_stack.item_id,
                    quantity: remaining,
                });
                return Ok(());
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(pub u32);

#[derive(Debug, Clone)]
pub struct ItemStack {
    /// `item_id < 256` represents blocks
    pub item_id: ItemId,
    /// `0 < quantity <= MAX_QUANTITY`
    quantity: u32,
}

impl ItemStack {
    pub const MAX_QUANTITY: u32 = 64;

    pub fn new(item_id: ItemId, quantity: u32) -> Result<Self> {
        if quantity == 0 || quantity > Self::MAX_QUANTITY {
            return Err("ItemStack quantity must be between 1 and MAX_QUANTITY".into());
        }
        Ok(Self { item_id, quantity })
    }

    pub fn quantity(&self) -> u32 {
        self.quantity
    }
}
