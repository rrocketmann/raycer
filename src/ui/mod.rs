use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui)
            .add_systems(Update, update_hud);
    }
}

#[derive(Component)]
pub struct SpeedText;

#[derive(Component)]
pub struct LapText;

#[derive(Component)]
pub struct RewardText;

fn setup_ui(mut commands: Commands) {
    // Speed display
    commands.spawn((
        Text::new("Speed: 0 km/h"),
        TextFont {
            font_size: 30.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Px(20.0),
            ..default()
        },
        SpeedText,
    ));

    // Lap counter
    commands.spawn((
        Text::new("Lap: 0"),
        TextFont {
            font_size: 30.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Px(20.0),
            ..default()
        },
        LapText,
    ));

    // Reward display
    commands.spawn((
        Text::new("Reward: 0.0"),
        TextFont {
            font_size: 30.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            right: Val::Px(20.0),
            ..default()
        },
        RewardText,
    ));
}

fn update_hud(
    mut speed_query: Query<&mut Text, With<SpeedText>>,
    mut lap_query: Query<&mut Text, With<LapText>>,
    mut reward_query: Query<&mut Text, With<RewardText>>,
) {
    if let Ok(mut text) = speed_query.single_mut() {
        text.0 = "Speed: 0 km/h".to_string();
    }
    if let Ok(mut text) = lap_query.single_mut() {
        text.0 = "Lap: 0".to_string();
    }
    if let Ok(mut text) = reward_query.single_mut() {
        text.0 = "Reward: 0.0".to_string();
    }
}
