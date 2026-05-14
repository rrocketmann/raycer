use bevy::prelude::*;

use crate::car::{Car, PlayerCar};
use crate::ui::{RewardText, SpeedText};

pub struct GymPlugin;

impl Plugin for GymPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GymState::default())
            .add_systems(Update, (accumulate_reward, update_hud));
    }
}

#[derive(Resource, Default)]
pub struct GymState {
    pub total_reward: f32,
    pub current_speed: f32,
    pub last_reward: f32,
    pub episode_steps: u32,
}

fn accumulate_reward(
    mut gym: ResMut<GymState>,
    car_query: Query<&Car, With<PlayerCar>>,
) {
    let Some(car) = car_query.iter().next() else {
        return;
    };

    gym.current_speed = car.speed;
    gym.last_reward = car.speed.abs() * 0.01;
    gym.total_reward += gym.last_reward;
    gym.episode_steps += 1;
}

fn update_hud(
    gym: Res<GymState>,
    mut speed_query: Query<&mut Text, (With<SpeedText>, Without<RewardText>)>,
    mut reward_query: Query<&mut Text, (With<RewardText>, Without<SpeedText>)>,
) {
    if gym.is_changed() {
        if let Ok(mut text) = speed_query.single_mut() {
            let speed_kmh = gym.current_speed * 3.6;
            text.0 = format!("Speed: {:.0} km/h", speed_kmh);
        }
        if let Ok(mut text) = reward_query.single_mut() {
            text.0 = format!("Reward: {:.1}", gym.total_reward);
        }
    }
}