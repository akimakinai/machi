use bevy::{camera::visibility::RenderLayers, input::keyboard::Key, prelude::*};

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<Pause>()
            .configure_sets(FixedUpdate, PausableSystems.run_if(in_state(Pause(false))))
            .configure_sets(Update, PausableSystems.run_if(in_state(Pause(false))))
            .add_systems(Update, toggle_pause)
            .add_systems(OnEnter(Pause(true)), pause_dim_on)
            .add_systems(OnExit(Pause(true)), pause_dim_off);
    }
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pause(pub bool);

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PausableSystems;

fn toggle_pause(
    pause: Res<State<Pause>>,
    mut next_pause: ResMut<NextState<Pause>>,
    keys: Res<ButtonInput<Key>>,
) {
    if keys.just_pressed(Key::Escape) {
        next_pause.set(Pause(!pause.0));
        debug!("Toggling pause: {next_pause:?}");
    }
}

#[derive(Component)]
struct PauseDimmingScreen;

fn pause_dim_on(mut commands: Commands) {
    commands.spawn((
        PauseDimmingScreen,
        Name::new("Pause Dimming Screen"),
        Camera {
            order: 1,
            ..default()
        },
        Camera2d,
        RenderLayers::from_layers(&[1]),
    ));
    commands.spawn((
        PauseDimmingScreen,
        Name::new("Pause Dimming Screen Quad"),
        Sprite {
            color: Color::srgba(0.0, 0.0, 0.0, 0.8),
            custom_size: Some(Vec2::new(10000.0, 10000.0)),
            ..default()
        },
        Transform::default(),
        RenderLayers::from_layers(&[1]),
    ));
}

fn pause_dim_off(mut commands: Commands, query: Query<Entity, With<PauseDimmingScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
