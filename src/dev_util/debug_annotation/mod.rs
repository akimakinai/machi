pub mod target;

use bevy::{prelude::*, ui::UiSystems};

use target::AnnotTargetRect;

use crate::dev_util::debug_annotation::target::AnnotUpdateSystems;

pub struct DebugAnnotPlugin;

impl Plugin for DebugAnnotPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(target::TargetPlugin)
            // all UI systems in PostUpdate will run after `Prepare`
            .add_systems(
                PostUpdate,
                (update_annotation_ui, update_annot_info)
                    .after(AnnotUpdateSystems)
                    .before(UiSystems::Prepare),
            )
            .configure_sets(PostUpdate, AnnotUpdateSystems.before(UiSystems::Prepare));
    }
}

pub fn debug_annot_ui(target: Entity) -> impl Bundle {
    (
        DebugAnnotUi(target),
        children![
            DebugAnnotArea,
            (DebugAnnotInfoBox, children![Text::default()])
        ],
    )
}

#[derive(Component)]
#[relationship_target(relationship = DebugAnnotUi)]
pub struct AttachDebugAnnotUi(Entity);

#[derive(Component)]
#[relationship(relationship_target = AttachDebugAnnotUi)]
#[require(Node {
    flex_direction: FlexDirection::Column,
    ..default()
}, AnnotTargetRect)]
pub struct DebugAnnotUi(
    /// The entity this UI is annotating.
    pub Entity,
);

#[derive(Component)]
#[require(
    Node { border: UiRect::all(Val::Px(2.0)), ..default() },
    BorderColor::from(Color::srgba(0.0, 0.0, 0.0, 0.8)),
)]
struct DebugAnnotArea;

#[derive(Component)]
#[require(Node, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.3)))]
struct DebugAnnotInfoBox;

fn update_annotation_ui(
    mut query: Query<(Entity, &AnnotTargetRect, &Children, &Visibility), With<DebugAnnotUi>>,
    annotation_area: Query<(), With<DebugAnnotArea>>,
    mut nodes: Query<&mut Node>,
    ui_camera: DefaultUiCamera,
    camera: Query<&Camera>,
) -> Result<()> {
    let Some(ui_camera) = ui_camera.get() else {
        return Ok(());
    };

    let ui_camera_viewport_pos = camera
        .get(ui_camera)?
        .viewport
        .as_ref()
        .map(|v| v.physical_position)
        .unwrap_or_default()
        .as_vec2();

    for (entity, target_rect, children, vis) in &mut query {
        if vis == Visibility::Hidden {
            continue;
        }
        if let Ok(node) = nodes.get(entity)
            && node.display == Display::None
        {
            continue;
        }

        let Some(target_rect) = target_rect.0 else {
            continue;
        };

        for child in children.iter() {
            if annotation_area.contains(child) {
                let viewport_pos = Rect {
                    min: target_rect.min - ui_camera_viewport_pos,
                    max: target_rect.max - ui_camera_viewport_pos,
                };

                let mut node = nodes.get_mut(child)?;
                node.width = Val::Px(viewport_pos.width());
                node.height = Val::Px(viewport_pos.height());

                let mut node = nodes.get_mut(entity)?;
                node.left = Val::Px(viewport_pos.min.x);
                node.top = Val::Px(viewport_pos.min.y);
            }
        }
    }

    Ok(())
}

fn update_annot_info(
    annot_ui: Query<(&DebugAnnotUi, &Children)>,
    info_boxes: Query<&Children, With<DebugAnnotInfoBox>>,
    mut texts: Query<&mut Text>,
) {
    for (&DebugAnnotUi(target_id), children) in &annot_ui {
        for child in children.iter() {
            if let Ok(info_children) = info_boxes.get(child) {
                for info_child in info_children.iter() {
                    if let Ok(mut text) = texts.get_mut(info_child) {
                        text.0 = format!("Entity: {target_id:?}");
                    }
                }
            }
        }
    }
}
