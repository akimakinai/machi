use bevy::prelude::*;

use crate::item::ItemStack;

#[derive(Component, Debug, Clone)]
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
}

impl Inventory {
    /// Returns `Err(remaining)` if there is not enough space.
    pub fn add_item_stack(&mut self, item_stack: ItemStack) -> Result<(), ItemStack> {
        let mut remaining = item_stack.quantity();
        for slot in self.slots.iter_mut() {
            if let Some(existing_stack) = slot
                && existing_stack.item_id == item_stack.item_id
            {
                let can_add = ItemStack::MAX_QUANTITY - existing_stack.quantity();
                let to_add = remaining.min(can_add);
                existing_stack
                    .set_quantity(existing_stack.quantity() + to_add)
                    .unwrap();
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
                *slot = Some(ItemStack::new(item_stack.item_id, remaining).unwrap());
                return Ok(());
            }
        }
        if remaining > 0 {
            Err(ItemStack::new(item_stack.item_id, remaining).unwrap())
        } else {
            Ok(())
        }
    }
}
