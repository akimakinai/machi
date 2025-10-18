use bevy::prelude::*;

pub struct HotbarPlugin;

impl Plugin for HotbarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hotbar);
    }
}

fn setup_hotbar(mut commands: Commands) {
    commands
        .spawn((
            Name::new("Hotbar UI"),
            Node {
                width: percent(80.0),
                height: px(50.0),
                position_type: PositionType::Absolute,
                bottom: px(10.0),
                left: percent(10.0),
                justify_content: JustifyContent::SpaceEvenly,
                ..default()
            },
        ))
        .with_children(|parent| {
            for i in 0..9 {
                parent.spawn((
                    Name::new(format!("Hotbar Slot {}", i + 1)),
                    Node {
                        width: px(40.0),
                        height: px(40.0),
                        border: UiRect::all(px(2.0)),
                        ..default()
                    },
                ));
            }
        });
}
