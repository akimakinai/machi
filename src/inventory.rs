use bevy::prelude::*;

use crate::item::item_core::ItemStack;

#[derive(Component, Debug, Clone)]
pub struct Inventory {
    pub size: u32,
}

impl Inventory {
    /// Returns `Ok(remaining)` where `remaining` is the number of items that could not be added.
    pub fn add_item_stack(inventory: Entity, item_stack: ItemStack) -> impl Command<Result<u32>> {
        move |world: &mut World| {
            let children = world
                .get::<Children>(inventory)
                .ok_or("Inventory entity has no Children component")?
                .to_vec();

            let mut remaining = item_stack.quantity();
            for &slot in &children {
                let mut slot = world
                    .get_mut::<ItemStack>(slot)
                    .ok_or("Inventory slot entity has no ItemStack component")?;

                if slot.item_id == item_stack.item_id {
                    let can_add = ItemStack::MAX_QUANTITY - slot.quantity();
                    let to_add = remaining.min(can_add);
                    let quantity = slot.quantity();
                    slot.set_quantity(quantity + to_add).unwrap();
                    remaining -= to_add;
                    if remaining == 0 {
                        break;
                    }
                }
            }
            if remaining == 0 {
                return Ok(0);
            }

            let inventory_size = world
                .get::<Inventory>(inventory)
                .ok_or("entity has no Inventory component")?
                .size;

            if children.len() < inventory_size as usize {
                // If the slot is empty, we can put all remaining items here
                // since `remaining <= MAX_QUANTITY`.
                world.spawn((
                    ItemStack::new(item_stack.item_id, remaining).unwrap(),
                    ChildOf(inventory),
                ));
                return Ok(0);
            }

            Ok(remaining)
        }
    }
}
