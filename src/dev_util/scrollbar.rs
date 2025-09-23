use bevy::prelude::*;

pub struct ScrollbarPlugin;

impl Plugin for ScrollbarPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_handle)
            .add_systems(Update, update_scrollbar);
    }
}

#[derive(Component)]
#[require(Node {
    position_type: PositionType::Absolute,
        // right: Val::Px(0.0),
        // top: Val::Px(0.0),
        // width: Val::Px(16.0),
        // height: Val::Percent(100.0),
    ..default()
}, BackgroundColor(Color::srgb(0.5, 0.5, 0.5)))]
pub struct Scrollbar {
    pub target: Entity,
    pub kind: ScrollbarKind,
}

pub enum ScrollbarKind {
    Vertical,
    #[expect(dead_code)]
    Horizontal,
}

#[derive(Component)]
#[require(Node {
    position_type: PositionType::Absolute,
    ..default()
}, BackgroundColor(Color::srgb(1.0, 1.0, 1.0)))]
struct ScrollbarHandle;

fn spawn_handle(on: On<Add, Scrollbar>, scrollbars: Query<&Scrollbar>, mut commands: Commands) {
    debug!("Spawn handle for scrollbar {:?}", on.event_target());
    let Ok(scrollbar) = scrollbars.get(on.event_target()) else {
        return;
    };

    let mut node = Node {
        position_type: PositionType::Absolute,
        ..default()
    };
    match scrollbar.kind {
        ScrollbarKind::Vertical => {
            node.width = percent(100.0);
        }
        ScrollbarKind::Horizontal => {
            node.height = percent(100.0);
        }
    }

    commands
        .entity(on.event_target())
        .with_child((ScrollbarHandle, node));
}

fn update_scrollbar(
    scrollbars: Query<(Entity, &Scrollbar, &Children)>,
    computed_nodes: Query<&ComputedNode>,
    mut nodes: Query<&mut Node>,
    handles: Query<(), With<ScrollbarHandle>>,
) {
    for (id, scrollbar, chilren) in &scrollbars {
        let Ok(computed) = computed_nodes.get(scrollbar.target) else {
            continue;
        };

        let (offset, size) = match scrollbar.kind {
            ScrollbarKind::Vertical => (
                computed.scroll_position.y / computed.content_size.y,
                computed.size.y / computed.content_size.y,
            ),
            ScrollbarKind::Horizontal => (
                computed.scroll_position.x / computed.content_size.x,
                computed.size.x / computed.content_size.x,
            ),
        };
        if offset.is_nan() || size.is_nan() {
            continue;
        }

        let Ok(mut node) = nodes.get_mut(id) else {
            continue;
        };
        match scrollbar.kind {
            ScrollbarKind::Vertical => {
                node.reborrow()
                    .map_unchanged(|node| &mut node.width)
                    .set_if_neq(Val::Px(computed.scrollbar_size.x));
                node.reborrow()
                    .map_unchanged(|node| &mut node.height)
                    .set_if_neq(Val::Percent(100.0));
                node.reborrow()
                    .map_unchanged(|node| &mut node.right)
                    .set_if_neq(Val::Px(0.0));
                node.reborrow()
                    .map_unchanged(|node| &mut node.top)
                    .set_if_neq(Val::Px(0.0));
            }
            ScrollbarKind::Horizontal => {
                node.reborrow()
                    .map_unchanged(|node| &mut node.height)
                    .set_if_neq(Val::Px(computed.scrollbar_size.y));
                node.reborrow()
                    .map_unchanged(|node| &mut node.width)
                    .set_if_neq(Val::Percent(100.0));
                node.reborrow()
                    .map_unchanged(|node| &mut node.bottom)
                    .set_if_neq(Val::Px(0.0));
                node.reborrow()
                    .map_unchanged(|node| &mut node.left)
                    .set_if_neq(Val::Px(0.0));
            }
        };

        for child in chilren.iter() {
            if !handles.contains(child) {
                continue;
            }
            let Ok(mut handle_node) = nodes.get_mut(child) else {
                continue;
            };

            match scrollbar.kind {
                ScrollbarKind::Vertical => {
                    handle_node.reborrow().map_unchanged(|node| &mut node.top)
                }
                ScrollbarKind::Horizontal => {
                    handle_node.reborrow().map_unchanged(|node| &mut node.left)
                }
            }
            .set_if_neq(Val::Percent(offset * 100.0));

            match scrollbar.kind {
                ScrollbarKind::Vertical => handle_node
                    .reborrow()
                    .map_unchanged(|node| &mut node.height),
                ScrollbarKind::Horizontal => {
                    handle_node.reborrow().map_unchanged(|node| &mut node.width)
                }
            }
            .set_if_neq(Val::Percent(size * 100.0));
        }
    }
}
