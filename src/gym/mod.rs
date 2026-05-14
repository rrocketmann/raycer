use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub struct GymPlugin;

impl Plugin for GymPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GymEnv>()
            .add_systems(Update, (step_env, reset_env));
    }
}

#[derive(Resource)]
pub struct GymEnv {
    pub running: bool,
    pub episode: usize,
    pub step: usize,
    pub max_steps: usize,
    pub reward_config: RewardConfig,
    pub last_reward: f32,
    pub total_reward: f32,
}

impl Default for GymEnv {
    fn default() -> Self {
        Self {
            running: false,
            episode: 0,
            step: 0,
            max_steps: 1000,
            reward_config: RewardConfig::default(),
            last_reward: 0.0,
            total_reward: 0.0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RewardConfig {
    pub progress_weight: f32,
    pub speed_weight: f32,
    pub collision_penalty: f32,
    pub time_penalty: f32,
    pub off_road_penalty: f32,
    pub finish_bonus: f32,
}

impl Default for RewardConfig {
    fn default() -> Self {
        Self {
            progress_weight: 1.0,
            speed_weight: 0.1,
            collision_penalty: -5.0,
            time_penalty: -0.01,
            off_road_penalty: -1.0,
            finish_bonus: 100.0,
        }
    }
}

#[derive(Clone)]
pub struct Observation {
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Quat,
    pub angular_velocity: Vec3,
    pub speed: f32,
    pub track_progress: f32,
}

impl Observation {
    pub fn to_vec(&self) -> Vec<f32> {
        vec![
            self.position.x,
            self.position.y,
            self.position.z,
            self.velocity.x,
            self.velocity.y,
            self.velocity.z,
            self.rotation.x,
            self.rotation.y,
            self.rotation.z,
            self.rotation.w,
            self.angular_velocity.x,
            self.angular_velocity.y,
            self.angular_velocity.z,
            self.speed,
            self.track_progress,
        ]
    }
}

#[derive(Clone)]
pub struct Action {
    pub throttle: f32,
    pub steer: f32,
    pub brake: f32,
}

pub fn step_env(
    mut env: ResMut<GymEnv>,
    // TODO: Add car and track queries
) {
    if !env.running {
        return;
    }

    env.step += 1;
    env.total_reward += env.last_reward;

    if env.step >= env.max_steps {
        env.running = false;
        env.episode += 1;
    }
}

pub fn reset_env(
    mut env: ResMut<GymEnv>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        env.running = false;
        env.episode += 1;
        env.step = 0;
        env.total_reward = 0.0;
        env.last_reward = 0.0;
    }
}
