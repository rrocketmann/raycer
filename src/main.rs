use bevy::prelude::*;
use avian3d::dynamics::solver::SolverConfig;
use avian3d::prelude::*;
use rand::Rng;

use crate::car::{PlayerCar, Health, AiCar, ExplosionTimer};

mod ai;
mod blaster;
mod car;
mod track;
mod ui;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
enum GameState {
    #[default]
    Loading,
    PreGame,
    Playing,
    Eliminated,
}

#[derive(Resource)]
pub struct RubberBullets {
    pub enabled: bool,
    pub random: bool,
}

impl Default for RubberBullets {
    fn default() -> Self { Self { enabled: false, random: false } }
}

#[derive(Resource)]
pub struct MaxHealthPoints {
    pub hp: u8,
    pub random: bool,
}

impl Default for MaxHealthPoints {
    fn default() -> Self { Self { hp: 3, random: false } }
}

#[derive(Resource, Default)]
pub struct GameOutcome(pub bool);

#[derive(Resource, Default)]
pub struct PendingState(Option<GameState>);

#[derive(Resource)]
struct LoadingAssets {
    handles: Vec<Handle<Scene>>,
}

fn enter_pregame(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<GameState>>,
    loading: Option<Res<LoadingAssets>>,
) {
    let Some(loading) = loading else {
        let paths = [
            car::CAR_DEFS[0].path,
            blaster::BLASTER_DEFS[0].path,
            "Map.glb",
        ];
        let handles: Vec<Handle<Scene>> = paths
            .iter()
            .map(|p| asset_server.load(GltfAssetLabel::Scene(0).from_asset(*p)))
            .collect();
        commands.insert_resource(LoadingAssets { handles });
        return;
    };

    if loading.handles.iter().all(|h| asset_server.is_loaded_with_dependencies(h)) {
        next_state.set(GameState::PreGame);
        commands.remove_resource::<LoadingAssets>();
    }
}

#[derive(Resource)]
pub struct AiEnemyCount {
    pub count: usize,
    pub random: bool,
}

impl Default for AiEnemyCount {
    fn default() -> Self { Self { count: 3, random: false } }
}

fn apply_pending_state(
    mut pending: ResMut<PendingState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Some(state) = pending.0.take() {
        next_state.set(state);
    }
}

fn check_game_state(
    mut commands: Commands,
    mut pending: ResMut<PendingState>,
    mut outcome: ResMut<GameOutcome>,
    player_query: Query<(Entity, &Health, &Position), With<PlayerCar>>,
    ai_query: Query<(), With<AiCar>>,
    exploding_query: Query<&ExplosionTimer>,
    enemy_count: Res<AiEnemyCount>,
) {
    for (entity, health, pos) in player_query.iter() {
        if pos.0.y < -20.0 {
            outcome.0 = false;
            commands.entity(entity).insert((
                ExplosionTimer(Timer::from_seconds(0.4, TimerMode::Once)),
                LinearVelocity::ZERO,
                AngularVelocity::ZERO,
            ));
            pending.0 = Some(GameState::Eliminated);
            return;
        }
        if health.0 == 0 && exploding_query.get(entity).is_err() {
            outcome.0 = false;
            commands.entity(entity).insert((
                ExplosionTimer(Timer::from_seconds(0.4, TimerMode::Once)),
                LinearVelocity::ZERO,
                AngularVelocity::ZERO,
            ));
            pending.0 = Some(GameState::Eliminated);
            return;
        }
    }
    if enemy_count.count > 0 && ai_query.iter().count() == 0 {
        for (_, player_health, _) in player_query.iter() {
            if player_health.0 > 0 {
                outcome.0 = true;
                pending.0 = Some(GameState::Eliminated);
            }
        }
    }
}

fn resolve_random_options(
    mut enemy_count: ResMut<AiEnemyCount>,
    mut max_hp: ResMut<MaxHealthPoints>,
    mut rubber_bullets: ResMut<RubberBullets>,
) {
    if enemy_count.random {
        enemy_count.count = rand::rng().random_range(0..=10);
        enemy_count.random = false;
    }
    if max_hp.random {
        max_hp.hp = rand::rng().random_range(2..=10);
        max_hp.random = false;
    }
    if rubber_bullets.random {
        rubber_bullets.enabled = rand::rng().random_bool(0.5);
        rubber_bullets.random = false;
    }
}

fn main() {
    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: "raycer".into(),
            canvas: Some("#raycer".into()),
            ..default()
        }),
        ..default()
    };

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(window_plugin))
        .add_plugins(PhysicsPlugins::default())
        .insert_resource(Gravity(Vec3::NEG_Y * 120.0))
        .insert_resource(SubstepCount(12))
        .insert_resource(SolverConfig {
            contact_damping_ratio: 15.0,
            max_overlap_solve_speed: 8.0,
            ..default()
        })
        .add_plugins(bevy_egui::EguiPlugin::default())
        .init_state::<GameState>()
        .init_resource::<AiEnemyCount>()
        .init_resource::<RubberBullets>()
        .init_resource::<MaxHealthPoints>()
        .init_resource::<GameOutcome>()
        .init_resource::<PendingState>()
        .add_systems(OnEnter(GameState::Playing), resolve_random_options)
        .add_systems(Update, enter_pregame.run_if(in_state(GameState::Loading)))
        .add_systems(Update, check_game_state.run_if(in_state(GameState::Playing)))
        .add_systems(Update, apply_pending_state)
        .add_plugins((ai::AiPlugin, blaster::BlasterPlugin, car::CarPlugin, track::TrackPlugin, ui::UiPlugin));

    #[cfg(feature = "dev")]
    app.add_plugins(avian3d::debug_render::PhysicsDebugPlugin);

    app.run();
}