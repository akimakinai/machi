use std::marker::PhantomData;

use bevy::{platform::collections::HashMap, prelude::*};

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemRegistry>()
            .add_systems(Startup, startup)
            .add_observer(on_use_dynamite);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(pub u32);

impl ItemId {
    pub fn is_block(&self) -> bool {
        self.0 < 256
    }
}

#[derive(Debug, Clone)]
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

    pub fn set_quantity(&mut self, quantity: u32) -> Result<()> {
        if quantity == 0 || quantity > Self::MAX_QUANTITY {
            return Err("ItemStack quantity must be between 1 and MAX_QUANTITY".into());
        }
        self.quantity = quantity;
        Ok(())
    }
}

#[derive(Resource, Default)]
pub struct ItemRegistry {
    /// Functions that trigger corresponding `ItemUse` events.
    /// This is for type erasure.
    on_use: HashMap<ItemId, fn(&mut Commands, Entity)>,
}

impl ItemRegistry {
    pub fn register_use<T: Component + 'static>(&mut self, item_id: ItemId) {
        self.on_use
            .insert(item_id, |commands: &mut Commands, player: Entity| {
                commands.trigger(ItemUse::<T>::new(player));
            });
    }

    pub fn use_item(&self, item_id: ItemId, commands: &mut Commands, player: Entity) {
        if let Some(on_use) = self.on_use.get(&item_id) {
            on_use(commands, player);
        }
    }
}

#[derive(Event)]
struct ItemUse<T> {
    player: Entity,
    marker: PhantomData<T>,
}

impl<T> ItemUse<T> {
    pub fn new(player: Entity) -> Self {
        Self {
            player,
            marker: PhantomData,
        }
    }
}

fn startup(mut reg: ResMut<ItemRegistry>) {
    reg.register_use::<Dynamite>(ItemId(256));
}

#[derive(Component)]
pub struct Dynamite;

fn on_use_dynamite(on: On<ItemUse<Dynamite>>) {
    info!("Dynamite used by {}", on.event().player);
}
