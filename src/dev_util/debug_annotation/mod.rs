mod target;

use bevy::prelude::*;

use target::CalloutTargetRect;

pub struct DebugCalloutPlugin;

impl Plugin for DebugCalloutPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(target::TargetPlugin);
    }
}

#[derive(Component)]
#[relationship_target(relationship = DebugCalloutUiOf)]
pub struct DebugCalloutUi(Entity);

#[derive(Component)]
#[relationship(relationship_target = DebugCalloutUi)]
#[require(Node)]
pub struct DebugCalloutUiOf(pub Entity);

#[derive(Component)]
#[require(Node)]
struct DebugCalloutArea;

fn update_callout_ui(
    mut query: Query<(&DebugCalloutUi, &Children)>,
    mut callout_area: Query<&mut Node, With<DebugCalloutArea>>,
    target_query: Query<&CalloutTargetRect>,
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

    for (callout_ui, children) in &mut query {
        let Ok(&CalloutTargetRect(target_rect)) = target_query.get(callout_ui.0) else {
            continue;
        };

        for child in children.iter() {
            if let Ok(mut node) = callout_area.get_mut(child) {
                let viewport_pos = Rect {
                    min: target_rect.min - ui_camera_viewport_pos,
                    max: target_rect.max - ui_camera_viewport_pos,
                };

                node.left = Val::Px(viewport_pos.min.x);
                node.top = Val::Px(viewport_pos.min.y);
                node.width = Val::Px(viewport_pos.width());
                node.height = Val::Px(viewport_pos.height());
            }
        }
    }

    Ok(())
}
