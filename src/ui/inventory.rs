use bevy::prelude::*;

use crate::{inventory::Inventory, ui::block_icon::BlockIconMaterial};

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
    pub chest_inventory: Option<Entity>,
    pub inventory: Entity,
    /// Treats the last N slots in `inventory` as hotbar slots
    pub hotbar: Option<u32>,
}

#[derive(Component)]
struct InventoryUiSlot(usize);

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
    mut block_icon_mats: ResMut<Assets<BlockIconMaterial>>,
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
                    for i in 0..data.slots.len() {
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
                                MaterialNode(block_icon_mats.add(BlockIconMaterial {
                                        // icon: Default::default(),
                                    })),
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(0.0),
                                    left: Val::Px(0.0),
                                    width: percent(100.0),
                                    height: percent(100.0),
                                    ..default()
                                },
                            ));
                            slot.spawn((
                                Name::new("Count"),
                                Node {
                                    position_type: PositionType::Absolute,
                                    right: Val::Px(4.0),
                                    bottom: Val::Px(4.0),
                                    ..default()
                                },
                                Text::new(String::new()),
                                TextColor(Color::BLACK),
                                TextShadow {
                                    offset: Vec2::splat(2.),
                                    color: Color::srgba(0., 0., 0., 0.75),
                                },
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
    roots: Query<(Entity, Ref<InventoryUiRoot>)>,
    slots: Query<&InventoryUiSlot>,
    inventories: Query<Ref<Inventory>>,
    children: Query<&Children>,
    mut texts: Query<&mut Text>,
) -> Result<()> {
    for (root_id, root) in &roots {
        let inventory = inventories.get(root.inventory)?;
        if !root.is_added() && !inventory.is_changed() {
            continue;
        }

        for child in children.iter_descendants(root_id) {
            let Ok(slot) = slots.get(child) else {
                continue;
            };

            let num = inventory.slots[slot.0]
                .as_ref()
                .map(|s| s.quantity.to_string())
                .unwrap_or_default();

            for schild in children.get(child)?.iter() {
                if let Ok(mut text) = texts.get_mut(schild) {
                    text.0 = num;
                    break;
                }
            }
        }
    }

    Ok(())
}
