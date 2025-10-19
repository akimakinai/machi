use bevy::{ecs::relationship::RelatedSpawner, input::mouse::AccumulatedMouseScroll, prelude::*};

use crate::{inventory::Inventory, ui::item_icon::ItemIconNode};

pub struct HotbarPlugin;

impl Plugin for HotbarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_hotbar)
            .add_systems(Update, update_hotbar_active_slot);
    }
}

#[derive(Component)]
pub struct Hotbar {
    pub inventory: Entity,
    pub active_slot: u8,
}

impl Hotbar {
    fn new(inventory: Entity) -> Self {
        Self {
            inventory,
            active_slot: 0,
        }
    }
}

pub fn build_hotbar(
    In(inventory_id): In<Entity>,
    inventories: Query<&Inventory>,
    mut commands: Commands,
) -> Result<()> {
    let hotbar_num = inventories.get(inventory_id)?.hotbar.unwrap_or(0);
    commands.spawn((
        Name::new("Hotbar UI"),
        Hotbar::new(inventory_id),
        Node {
            width: percent(80.0),
            height: px(50.0),
            position_type: PositionType::Absolute,
            bottom: px(10.0),
            left: percent(10.0),
            justify_content: JustifyContent::SpaceEvenly,
            ..default()
        },
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            for i in 0..hotbar_num {
                parent
                    .spawn((
                        ItemIconNode(None),
                        Name::new(format!("Hotbar Slot {}", i + 1)),
                        Node {
                            width: px(48.0),
                            height: px(48.0),
                            border: UiRect::all(px(2.0)),
                            ..default()
                        },
                    ))
                    .with_child((
                        Text::default(),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        Name::new("Item Count"),
                        Node {
                            position_type: PositionType::Absolute,
                            right: px(0),
                            bottom: px(0),
                            ..default()
                        },
                    ));
            }
        })),
    ));
    Ok(())
}

fn update_hotbar(
    hotbars: Query<(&Hotbar, &Children)>,
    inventories: Query<&Inventory>,
    mut item_icons: Query<(&ItemIconNode, &mut BorderColor, &Children)>,
    mut texts: Query<&mut Text>,
    mut commands: Commands,
) -> Result<()> {
    for (hotbar, children) in hotbars.iter() {
        let Ok(inventory) = inventories.get(hotbar.inventory) else {
            continue;
        };

        let hotbar_num = inventory.hotbar.unwrap_or(0) as usize;

        for i in 0..hotbar_num {
            let Some(&child) = children.get(i) else {
                error!("Hotbar slot {} missing", i);
                break;
            };

            let Some(slot) = inventory.slots.get(i) else {
                error!(?inventory, "Hotbar slot {} out of bounds", i);
                continue;
            };

            let item_id = slot.as_ref().map(|is| is.item_id);
            let item_num = slot.as_ref().map(|is| is.quantity()).unwrap_or(0);

            let (item_icon, mut border_color, children) = item_icons.get_mut(child)?;

            if hotbar.active_slot as usize == i {
                border_color.set_if_neq(BorderColor::all(Color::BLACK));
            } else {
                border_color.set_if_neq(BorderColor::all(Color::WHITE));
            }

            if item_icon.0 != item_id {
                commands.entity(child).insert(ItemIconNode(item_id));
            }

            let Some(&text_id) = children.first() else {
                continue;
            };

            texts.get_mut(text_id)?.0 = if item_num > 1 {
                item_num.to_string()
            } else {
                String::new()
            };
        }
    }

    Ok(())
}

fn update_hotbar_active_slot(
    mut hotbars: Query<&mut Hotbar>,
    inventories: Query<&Inventory>,
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<AccumulatedMouseScroll>,
) -> Result<()> {
    for mut hotbar in &mut hotbars {
        let hotbar_size = inventories.get(hotbar.inventory)?.hotbar.unwrap_or(0) as u8;
        if hotbar_size == 0 {
            continue;
        }

        for i in 0..hotbar_size {
            if let Some(key) = digit_key(i + 1)
                && keys.just_pressed(key)
            {
                hotbar.active_slot = i;
            }
        }

        let delta_y = scroll.delta.y;
        if delta_y > 0.0 {
            hotbar.active_slot = (hotbar.active_slot + hotbar_size - 1) % hotbar_size;
        } else if delta_y < 0.0 {
            hotbar.active_slot = (hotbar.active_slot + 1) % hotbar_size;
        }
    }

    Ok(())
}

fn digit_key(n: u8) -> Option<KeyCode> {
    match n {
        0 => Some(KeyCode::Digit0),
        1 => Some(KeyCode::Digit1),
        2 => Some(KeyCode::Digit2),
        3 => Some(KeyCode::Digit3),
        4 => Some(KeyCode::Digit4),
        5 => Some(KeyCode::Digit5),
        6 => Some(KeyCode::Digit6),
        7 => Some(KeyCode::Digit7),
        8 => Some(KeyCode::Digit8),
        9 => Some(KeyCode::Digit9),
        _ => None,
    }
}
