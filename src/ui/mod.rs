use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui)
            .add_systems(Startup, fix_ui_camera_order.after(setup_ui));
    }
}

#[derive(Component)]
pub struct SpeedText;

#[derive(Component)]
pub struct RewardText;

#[derive(Component)]
pub struct HudCamera;

fn setup_ui(mut commands: Commands) {
    // HUD camera (order set to 1 in fix_ui_camera_order system)
    commands.spawn((Camera2d, HudCamera));

    // Speed display - bottom left
    commands.spawn((
        Text::new("Speed: 0 km/h"),
        TextFont {
            font_size: 36.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(30.0),
            left: Val::Px(30.0),
            ..default()
        },
        SpeedText,
    ));

    // Reward display - top right
    commands.spawn((
        Text::new("Reward: 0.0"),
        TextFont {
            font_size: 36.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.9, 0.2)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(30.0),
            right: Val::Px(30.0),
            ..default()
        },
        RewardText,
    ));

    // Controls hint
    commands.spawn((
        Text::new("WASD / Arrows: Drive  |  Space: Brake  |  R: Reset"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.7, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(50.0),
            ..default()
        },
    ));
}

fn fix_ui_camera_order(mut query: Query<&mut Camera, With<HudCamera>>) {
    for mut cam in query.iter_mut() {
        cam.order = 1;
    }
}