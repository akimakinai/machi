use bevy::prelude::*;

use crate::inventory::{Inventory, ItemStack};

pub struct InventoryUiPlugin;

impl Plugin for InventoryUiPlugin {
    fn build(&self, app: &mut App) {}
}

#[derive(Component)]
#[require(Node)]
pub struct InventoryUiRoot {
    // TODO: use moonshine-kind
    pub chest_inventory: Option<Entity>,
    pub inventory: Entity,
    /// Treats the last N slots in `inventory` as hotbar slots
    pub hotbar: Option<u32>,
}

pub fn build_inventory_root(
    In(inventory): In<Entity>,
    mut commands: Commands,
    inventories: Query<(NameOrEntity, &Inventory)>,
) {
    let slot_size = 60.0;
    let slot_gap = 8.0;

    let (name, data) = inventories
        .get(inventory)
        .expect("Inventory entity does not exist");

    commands
        .spawn((
            Name::new(format!("Inventory UI Root for {}", name)),
            InventoryUiRoot {
                chest_inventory: None,
                inventory,
                hotbar: None,
            },
            Node {
                width: percent(80.0),
                height: percent(80.0),
                position_type: PositionType::Absolute,
                left: Val::Percent(10.0),
                top: Val::Percent(10.0),
                border: UiRect::all(Val::Px(2.0)),
                align_items: AlignItems::FlexStart,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.8)),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Grid"),
                    Node {
                        display: Display::Flex,
                        flex_wrap: FlexWrap::Wrap,
                        // width: Val::Px(columns as f32 * (slot_size + slot_gap)),
                        ..default()
                    },
                ))
                .with_children(|grid| {
                    for i in 0..data.slots.len() {
                        let mut slot = grid.spawn((
                            Node {
                                width: Val::Px(slot_size),
                                height: Val::Px(slot_size),
                                margin: UiRect::all(Val::Px(slot_gap * 0.5)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                        ));
                        slot.with_children(|slot| {
                            let mut count = slot.spawn((
                                Name::new("Count"),
                                Node {
                                    position_type: PositionType::Absolute,
                                    right: Val::Px(4.0),
                                    bottom: Val::Px(4.0),
                                    ..default()
                                },
                            ));
                            if let Some(stack) = &data.slots[i] {
                                count.insert(Text::new(stack.quantity.to_string()));
                            }
                        });
                    }
                });
        });
}
