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
    mut texts: Query<(
        Option<&SpeedText>,
        Option<&LapText>,
        Option<&RewardText>,
        &mut Text,
    )>,
) {
    for (speed, lap, reward, mut text) in texts.iter_mut() {
        if speed.is_some() {
            text.0 = "Speed: 0 km/h".to_string();
        } else if lap.is_some() {
            text.0 = "Lap: 0".to_string();
        } else if reward.is_some() {
            text.0 = "Reward: 0.0".to_string();
        }
    }
}
