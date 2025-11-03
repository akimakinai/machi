use bevy::prelude::*;

use crate::{inventory::Inventory, item::item_core::ItemStack, ui::item_icon::ItemIconNode};

pub struct InventoryUiPlugin;

impl Plugin for InventoryUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<InventoryState>()
            .add_systems(
                Update,
                (inventory_toggle, update_inventory_visibility).chain(),
            )
            .add_systems(Update, update_inventory_slots);
    }
}

#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone)]
pub enum InventoryState {
    Open,
    #[default]
    Close,
}

#[derive(Component)]
#[require(Node)]
pub struct InventoryUiRoot {
    // TODO: use moonshine-kind
    #[expect(dead_code)]
    pub chest_inventory: Option<Entity>,
    pub inventory: Entity,
}

#[derive(Component)]
struct InventoryUiSlot(u32);

#[derive(Component)]
struct SlotBlockIcon;

// #E2A16F
const INVENTORY_BACKGROUND: Color = Color::srgba_u8(0xE2, 0xA1, 0x6F, 0xC0);
// #FFF0DD
const INVENTORY_SLOT_BACKGROUND: Color = Color::srgba_u8(0xFF, 0xF0, 0xDD, 0xFF);
// #D1D3D4
const INVENTORY_BORDER_TOP: Color = Color::srgba_u8(0xD1, 0xD3, 0xD4, 0xFF);
// #86B0BD
const INVENTORY_BORDER_BOTTOM: Color = Color::srgba_u8(0x86, 0xB0, 0xBD, 0xFF);

pub fn build_inventory_root(
    In(inventory): In<Entity>,
    mut commands: Commands,
    inventories: Query<(NameOrEntity, &Inventory)>,
) {
    // let uv_debug_image = images.add(uv_debug_texture());

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
            },
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                left: Val::Percent(10.0),
                top: Val::Percent(10.0),
                width: percent(80.0),
                height: percent(80.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                border: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BorderRadius::all(px(6.0)),
            BorderColor {
                top: INVENTORY_BORDER_TOP,
                right: INVENTORY_BORDER_BOTTOM,
                bottom: INVENTORY_BORDER_BOTTOM,
                left: INVENTORY_BORDER_TOP,
            },
            BackgroundColor(INVENTORY_BACKGROUND),
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
                    for i in 0..data.size {
                        let mut slot = grid.spawn((
                            Name::new(format!("Slot {}", i)),
                            InventoryUiSlot(i),
                            Node {
                                width: Val::Px(slot_size),
                                height: Val::Px(slot_size),
                                margin: UiRect::all(Val::Px(slot_gap * 0.5)),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(INVENTORY_SLOT_BACKGROUND),
                            BorderColor::all(INVENTORY_SLOT_BACKGROUND.darker(0.2)),
                            BorderRadius::all(px(2.0)),
                        ));
                        slot.with_children(|slot| {
                            slot.spawn((
                                ItemIconNode(None),
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(0.0),
                                    left: Val::Px(0.0),
                                    width: percent(100.0),
                                    height: percent(100.0),
                                    ..default()
                                },
                                Visibility::Hidden,
                                SlotBlockIcon,
                            ));
                            slot.spawn((
                                Name::new("Count"),
                                Node {
                                    position_type: PositionType::Absolute,
                                    right: Val::Px(2.0),
                                    bottom: Val::Px(2.0),
                                    ..default()
                                },
                                Text::new(String::new()),
                                TextColor(Color::BLACK),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextBackgroundColor(Color::WHITE),
                            ));
                        });
                    }
                });
        });
}

fn update_inventory_visibility(
    state: Res<State<InventoryState>>,
    mut roots: Query<&mut Node, With<InventoryUiRoot>>,
) {
    let display = match state.get() {
        InventoryState::Open => Display::Flex,
        InventoryState::Close => Display::None,
    };
    for node in &mut roots {
        node.map_unchanged(|node| &mut node.display)
            .set_if_neq(display);
    }
}

fn inventory_toggle(
    state: Res<State<InventoryState>>,
    mut next: ResMut<NextState<InventoryState>>,
    key: Res<ButtonInput<KeyCode>>,
) {
    if key.just_pressed(KeyCode::KeyE) {
        next.set(match state.get() {
            InventoryState::Open => InventoryState::Close,
            InventoryState::Close => InventoryState::Open,
        });
    }
}

fn update_inventory_slots(
    q_root: Query<(Entity, Ref<InventoryUiRoot>)>,
    q_slot: Query<&InventoryUiSlot>,
    q_inventory: Query<Ref<Children>, With<Inventory>>,
    q_children: Query<&Children>,
    q_item: Query<&ItemStack>,
    q_item_icon: Query<&ItemIconNode>,
    mut q_text: Query<&mut Text>,
    mut q_block_icon: Query<&mut Visibility, With<SlotBlockIcon>>,
    mut commands: Commands,
) -> Result<()> {
    for (root_id, root) in &q_root {
        let inventory = q_inventory.get(root.inventory)?;
        if !root.is_added() && !inventory.is_changed() {
            continue;
        }

        for child in q_children.iter_descendants(root_id) {
            let Ok(slot) = q_slot.get(child) else {
                continue;
            };

            for schild in q_children.get(child)?.iter() {
                let item_stack_id = q_children.get(root.inventory)?.get(slot.0 as usize);
                let maybe_item = if let Some(&item_stack_id) = item_stack_id {
                    Some(q_item.get(item_stack_id)?)
                } else {
                    None
                };

                if let Ok(item_icon) = q_item_icon.get(schild) {
                    let maybe_item_id = maybe_item.map(|is| is.item_id);
                    if item_icon.0 != maybe_item_id {
                        commands.entity(schild).insert(ItemIconNode(maybe_item_id));
                    }
                }

                if let Ok(mut text) = q_text.get_mut(schild) {
                    text.0 = maybe_item
                        .map(|s| s.quantity().to_string())
                        .unwrap_or_default();
                }

                if let Ok(mut visibility) = q_block_icon.get_mut(schild) {
                    if maybe_item.is_some() {
                        visibility.set_if_neq(Visibility::Visible);
                    } else {
                        visibility.set_if_neq(Visibility::Hidden);
                    }
                }
            }
        }
    }

    Ok(())
}
