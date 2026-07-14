use bevy::prelude::*;
use avian3d::dynamics::solver::SolverConfig;
use avian3d::prelude::*;

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
pub struct RubberBullets(pub bool);

impl Default for RubberBullets {
    fn default() -> Self { Self(false) }
}

#[derive(Resource)]
pub struct MaxHealthPoints(pub u8);

impl Default for MaxHealthPoints {
    fn default() -> Self { Self(3) }
}

#[derive(Resource, Default)]
pub struct GameOutcome(pub bool);

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
pub struct AiEnemyCount(pub usize);

impl Default for AiEnemyCount {
    fn default() -> Self { Self(3) }
}

fn check_player_eliminated(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut outcome: ResMut<GameOutcome>,
    player_query: Query<(Entity, &Health), With<PlayerCar>>,
    exploding_query: Query<&ExplosionTimer>,
) {
    for (entity, health) in player_query.iter() {
        if health.0 == 0 && exploding_query.get(entity).is_err() {
            outcome.0 = false;
            commands.entity(entity).insert((
                ExplosionTimer(Timer::from_seconds(0.8, TimerMode::Once)),
                LinearVelocity::ZERO,
                AngularVelocity::ZERO,
            ));
            next_state.set(GameState::Eliminated);
        }
    }
}

fn check_win_condition(
    mut next_state: ResMut<NextState<GameState>>,
    mut outcome: ResMut<GameOutcome>,
    player_query: Query<&Health, With<PlayerCar>>,
    ai_query: Query<(), With<AiCar>>,
) {
    let Ok(player_health) = player_query.single() else { return };
    if player_health.0 == 0 { return; }
    if ai_query.iter().count() == 0 {
        outcome.0 = true;
        next_state.set(GameState::Eliminated);
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
        .add_systems(Update, enter_pregame.run_if(in_state(GameState::Loading)))
        .add_systems(Update, (check_player_eliminated, check_win_condition).run_if(in_state(GameState::Playing)))
        .add_plugins((ai::AiPlugin, blaster::BlasterPlugin, car::CarPlugin, track::TrackPlugin, ui::UiPlugin));

    #[cfg(feature = "dev")]
    app.add_plugins(avian3d::debug_render::PhysicsDebugPlugin);

    app.run();
}