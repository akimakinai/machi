use bevy::{ecs::relationship::RelatedSpawner, input::mouse::AccumulatedMouseScroll, prelude::*};

use crate::{inventory::Inventory, item::ItemStack, ui::item_icon::ItemIconNode};

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
    pub size: u8,
}

impl Hotbar {
    pub fn new(inventory: Entity, size: u8) -> Self {
        Self {
            inventory,
            active_slot: 0,
            size,
        }
    }
}
pub fn build_hotbar(In(hotbar): In<Hotbar>, mut commands: Commands) -> Result<()> {
    let hotbar_size = hotbar.size;
    commands.spawn((
        Name::new("Hotbar UI"),
        hotbar,
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
            for i in 0..hotbar_size {
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
                        BorderRadius::all(px(4.0)),
                        BackgroundColor(Color::BLACK),
                        Node {
                            width: px(15.0),
                            position_type: PositionType::Absolute,
                            right: px(0),
                            bottom: px(0),
                            ..default()
                        },
                        children![(
                            Text::default(),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                        )],
                    ));
            }
        })),
    ));
    Ok(())
}

fn update_hotbar(
    hotbars: Query<(&Hotbar, &Children)>,
    inventories: Query<&Children, With<Inventory>>,
    items: Query<&ItemStack>,
    mut item_icons: Query<(Entity, &ItemIconNode, &mut BorderColor)>,
    q_children: Query<&Children>,
    mut texts: Query<&mut Text>,
    mut commands: Commands,
) -> Result<()> {
    for (hotbar, hotbar_children) in hotbars.iter() {
        let Ok(children) = inventories.get(hotbar.inventory) else {
            continue;
        };

        for (i, child) in hotbar_children.iter().enumerate() {
            assert!(i < hotbar.size as usize);

            let (item_icon_id, item_icon, mut border_color) = item_icons.get_mut(child)?;

            if hotbar.active_slot as usize == i {
                border_color.set_if_neq(BorderColor::all(Color::BLACK));
            } else {
                border_color.set_if_neq(BorderColor::all(Color::WHITE));
            }

            let slot = children.get(i);
            let item_stack = slot.map(|&s| items.get(s)).transpose()?;

            let item_id = item_stack.map(|is| is.item_id);
            let item_num = item_stack.map(|is| is.quantity()).unwrap_or(0);

            if item_icon.0 != item_id {
                commands.entity(child).insert(ItemIconNode(item_id));
            }

            for id in q_children.iter_leaves(item_icon_id) {
                if let Ok(mut text_entity) = texts.get_mut(id) {
                    text_entity.0 = if item_num > 1 {
                        item_num.to_string()
                    } else {
                        String::new()
                    };
                    break;
                }
            }
        }
    }

    Ok(())
}

fn update_hotbar_active_slot(
    mut hotbars: Query<&mut Hotbar>,
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    mut cooldown: Local<Option<Timer>>,
) -> Result<()> {
    let cooldown = cooldown.get_or_insert_with(|| {
        let mut timer = Timer::from_seconds(0.2, TimerMode::Once);
        timer.finish();
        timer
    });
    cooldown.tick(time.delta());

    let delta_y = scroll.delta.y;
    if delta_y == 0.0 {
        cooldown.finish();
        return Ok(());
    }
    if cooldown.is_finished() {
        cooldown.reset();
    } else {
        return Ok(());
    }

    for mut hotbar in &mut hotbars {
        let hotbar_size = hotbar.size;
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
