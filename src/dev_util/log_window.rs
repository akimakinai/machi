use std::{fmt::Write as _, sync::LazyLock};

use bevy::{
    color::palettes::css,
    ecs::{query::QueryData, system::lifetimeless::Read},
    prelude::*,
    ui_widgets::{
        ControlOrientation, CoreScrollbarDragState, CoreScrollbarThumb, Scrollbar, ScrollbarPlugin,
    },
};
use tracing::Subscriber;
use tracing_subscriber::{Layer, layer::Context, registry::LookupSpan};

pub struct LogWindowPlugin;

impl Plugin for LogWindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ScrollbarPlugin)
            .init_resource::<LogTextId>()
            .add_systems(Startup, setup_log_window)
            .add_systems(
                Update,
                (
                    process_log_messages,
                    scroll_to_bottom,
                    detect_scrollbar_drag,
                ),
            );
    }
}

#[derive(Component)]
#[require(Node)]
struct LogWindowRoot {
    max_messages: Option<usize>,
}

#[derive(Component)]
#[require(Node)]
struct LogWindowMessageArea {
    keep_bottom: bool,
}

#[derive(Component)]
struct LogText(usize);

#[derive(Resource, Default)]
struct LogTextId(usize);

fn setup_log_window(mut commands: Commands) {
    commands
        .spawn((
            Name::new("Log Window"),
            LogWindowRoot {
                max_messages: Some(100),
            },
            Node {
                width: Val::Percent(80.0),
                height: Val::Percent(10.0),
                position_type: PositionType::Absolute,
                bottom: Val::Percent(0.0),
                left: Val::Percent(0.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        ))
        .with_children(|root| {
            const SCROLLBAR_WIDTH: f32 = 16.0;
            let target = root
                .spawn((
                    LogWindowMessageArea { keep_bottom: true },
                    Node {
                        padding: UiRect::all(Val::Px(4.0)),
                        overflow: Overflow::scroll_y(),
                        flex_direction: FlexDirection::Column,
                        scrollbar_width: SCROLLBAR_WIDTH,
                        ..default()
                    },
                ))
                .id();
            root.spawn(make_scrollbar(target, SCROLLBAR_WIDTH));
        });
}

fn make_scrollbar(target: Entity, width: f32) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            height: Val::Percent(100.0),
            min_width: Val::Px(width),
            ..default()
        },
        Scrollbar::new(target, ControlOrientation::Vertical, 20.0),
        BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.5)),
        children![(
            CoreScrollbarThumb,
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
            BorderColor::from(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            BorderRadius::MAX,
            Node {
                position_type: PositionType::Absolute,
                border: UiRect::all(Val::Px(2.0)),
                width: Val::Percent(100.0),
                ..default()
            },
        ),],
    )
}

#[derive(QueryData)]
struct IsClipped {
    clip: Option<Read<CalculatedClip>>,
    node: Read<ComputedNode>,
    transform: Read<UiGlobalTransform>,
}

impl IsClippedItem<'_, '_> {
    fn is_clipped(&self) -> bool {
        // from bevy_ui_render/src/ui_material_pipeline.rs
        let uinode_rect = Rect {
            min: Vec2::ZERO,
            max: self.node.size(),
        };
        let rect_size = uinode_rect.size();

        const QUAD_VERTEX_POSITIONS: [Vec2; 4] = [
            Vec2::new(-0.5, -0.5),
            Vec2::new(0.5, -0.5),
            Vec2::new(0.5, 0.5),
            Vec2::new(-0.5, 0.5),
        ];
        let positions = QUAD_VERTEX_POSITIONS
            .map(|pos| self.transform.transform_point2(pos * rect_size).extend(1.0));
        let positions_diff = if let Some(clip) = self.clip.map(|c| c.clip) {
            [
                Vec2::new(
                    f32::max(clip.min.x - positions[0].x, 0.),
                    f32::max(clip.min.y - positions[0].y, 0.),
                ),
                Vec2::new(
                    f32::min(clip.max.x - positions[1].x, 0.),
                    f32::max(clip.min.y - positions[1].y, 0.),
                ),
                Vec2::new(
                    f32::min(clip.max.x - positions[2].x, 0.),
                    f32::min(clip.max.y - positions[2].y, 0.),
                ),
                Vec2::new(
                    f32::max(clip.min.x - positions[3].x, 0.),
                    f32::min(clip.max.y - positions[3].y, 0.),
                ),
            ]
        } else {
            [Vec2::ZERO; 4]
        };
        let transformed_rect_size = self.transform.transform_vector2(rect_size);
        if self.transform.x_axis[1] == 0.0 {
            // Cull nodes that are completely clipped
            if positions_diff[0].x - positions_diff[1].x >= transformed_rect_size.x
                || positions_diff[1].y - positions_diff[2].y >= transformed_rect_size.y
            {
                return true;
            }
        }

        false
    }
}

fn scroll_to_bottom(
    mut commands: Commands,
    mut area: Query<(
        &LogWindowMessageArea,
        &ComputedNode,
        &mut ScrollPosition,
        &ChildOf,
        &Children,
    )>,
    log_window: Query<&LogWindowRoot>,
    texts: Query<&LogText, With<Text>>,
    is_clipped: Query<IsClipped>,
    cur_text_id: Res<LogTextId>,
) {
    // based on update_uinode_geometry_recursive in bevy_ui
    for (area, node, mut scroll_pos, child_of, children) in &mut area {
        if !area.keep_bottom {
            continue;
        }

        let log_window = log_window.get(child_of.parent()).unwrap();

        let mut decreased_content_size = Vec2::ZERO;

        if let Some(max_messages) = log_window.max_messages {
            for child in children.iter() {
                if let Ok(LogText(log_text_id)) = texts.get(child)
                    && cur_text_id.0 - log_text_id > max_messages
                    && let Ok(is_clipped) = is_clipped.get(child)
                    && is_clipped.is_clipped()
                {
                    decreased_content_size += is_clipped.node.size;
                    commands.entity(child).despawn();
                }
            }
        }

        let layout_size = node.size;
        let content_size = node.content_size - decreased_content_size;

        let max_possible_offset =
            (content_size - layout_size + node.scrollbar_size).max(Vec2::ZERO);

        scroll_pos.0 = scroll_pos.0.with_y(max_possible_offset.y);
    }
}

fn detect_scrollbar_drag(
    mut area: Query<(&mut LogWindowMessageArea, &ScrollPosition, &ComputedNode)>,
    scrollbars: Query<(&Scrollbar, &Children)>,
    scrollbar_state: Query<&CoreScrollbarDragState>,
) {
    const SCROLL_BOTTOM_THRESHOLD: f32 = 0.1;

    for (scrollbar, children) in &scrollbars {
        let Ok((mut area, scroll_pos, node)) = area.get_mut(scrollbar.target) else {
            continue;
        };
        let mut dragging = false;
        for child in children.iter() {
            if let Ok(state) = scrollbar_state.get(child) {
                if state.dragging {
                    dragging = true;
                    break;
                }
            }
        }
        if dragging {
            area.keep_bottom = false;
        } else {
            // set keep bottom if scrolled to bottom
            let diff = (node.content_size.y - node.size.y) - scroll_pos.0.y;
            if diff < SCROLL_BOTTOM_THRESHOLD {
                area.keep_bottom = true;
            }
        }
    }
}

#[derive(Message)]
enum LogWindowMessage {
    Add {
        level: String,
        origin: String,
        text: String,
    },
}

static PENDING_MESSAGES: LazyLock<std::sync::Mutex<Vec<LogWindowMessage>>> =
    LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

/// A tracing subscriber layer that captures log events and displays them in a UI window.
/// Based on `tracing-wasm`.
pub struct LogWindowLayer;

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for LogWindowLayer {
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::Id,
        ctx: Context<'_, S>,
    ) {
        let mut new_debug_record = StringRecorder::new();
        attrs.record(&mut new_debug_record);

        if let Some(span_ref) = ctx.span(id) {
            span_ref
                .extensions_mut()
                .insert::<StringRecorder>(new_debug_record);
        }
    }

    /// doc: Notifies this layer that a span with the given Id recorded the given values.
    fn on_record(&self, id: &tracing::Id, values: &tracing::span::Record<'_>, ctx: Context<'_, S>) {
        if let Some(span_ref) = ctx.span(id)
            && let Some(debug_record) = span_ref.extensions_mut().get_mut::<StringRecorder>()
        {
            values.record(debug_record);
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        if meta.fields().field("tracy.frame_mark").is_some() {
            return;
        }

        let mut recorder = StringRecorder::new();
        event.record(&mut recorder);
        let level = meta.level();
        let origin = meta
            .file()
            .and_then(|file| meta.line().map(|ln| format!("{}:{}", file, ln)))
            .unwrap_or_default();

        PENDING_MESSAGES
            .lock()
            .unwrap()
            .push(LogWindowMessage::Add {
                level: level.to_string(),
                origin,
                text: recorder.to_string(),
            });
    }
}

fn process_log_messages(
    mut commands: Commands,
    message_area: Query<Entity, With<LogWindowMessageArea>>,
    mut text_id: ResMut<LogTextId>,
) {
    let mut messages = PENDING_MESSAGES.lock().unwrap();
    if messages.is_empty() {
        return;
    }

    for message in messages.drain(..) {
        match message {
            LogWindowMessage::Add {
                level,
                origin,
                text,
            } => {
                let level_color = Color::from(match level.as_str() {
                    "TRACE" => css::LIGHT_GRAY,
                    "DEBUG" => css::SKY_BLUE,
                    "INFO" => css::GREEN,
                    "WARN" => css::YELLOW,
                    "ERROR" => css::RED,
                    _ => css::WHITE,
                });
                let font = TextFont {
                    font_size: 8.0,
                    ..default()
                };
                for id in &message_area {
                    commands.entity(id).with_child((
                        LogText(text_id.0),
                        Text::default(),
                        children![
                            (
                                TextSpan::new(level.clone()),
                                TextColor(level_color),
                                font.clone()
                            ),
                            (TextSpan::new(" "), font.clone()),
                            (TextSpan::new(origin.clone()), font.clone()),
                            (TextSpan::new(" "), font.clone()),
                            (TextSpan::new(text.clone()), font.clone()),
                        ],
                    ));
                    text_id.0 += 1;
                }
            }
        }
    }
}

// Taken from https://docs.rs/tracing-wasm/latest/src/tracing_wasm/lib.rs.html
struct StringRecorder {
    display: String,
    is_following_args: bool,
}
impl StringRecorder {
    fn new() -> Self {
        StringRecorder {
            display: String::new(),
            is_following_args: false,
        }
    }
}

impl tracing::field::Visit for StringRecorder {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            if !self.display.is_empty() {
                self.display = format!("{:?}\n{}", value, self.display)
            } else {
                self.display = format!("{:?}", value)
            }
        } else {
            if self.is_following_args {
                // following args
                writeln!(self.display).unwrap();
            } else {
                // first arg
                write!(self.display, " ").unwrap();
                self.is_following_args = true;
            }
            write!(self.display, "{} = {:?};", field.name(), value).unwrap();
        }
    }
}

impl core::fmt::Display for StringRecorder {
    fn fmt(&self, mut f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if !self.display.is_empty() {
            write!(&mut f, " {}", self.display)
        } else {
            Ok(())
        }
    }
}

impl core::default::Default for StringRecorder {
    fn default() -> Self {
        StringRecorder::new()
    }
}
