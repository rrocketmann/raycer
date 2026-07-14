use bevy::prelude::*;
use avian3d::dynamics::solver::SolverConfig;
use avian3d::prelude::*;

use crate::car::{PlayerCar, Health, ExplosionTimer};
use crate::ui::UiAction;

mod ai;
mod blaster;
mod car;
mod track;
mod ui;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Loading,
    PreGame,
    Playing,
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
pub struct PendingState(pub Option<GameState>);

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

fn apply_pending_state(
    mut pending: ResMut<PendingState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Some(state) = pending.0.take() {
        next_state.set(state);
    }
}

fn handle_ui_actions(
    mut action: ResMut<UiAction>,
    mut pending: ResMut<PendingState>,
) {
    if action.0 == 1 {
        pending.0 = Some(GameState::Playing);
    }
    action.0 = 0;
}

fn check_game_state(
    mut commands: Commands,
    mut pending: ResMut<PendingState>,
    player_query: Query<(Entity, &Health), With<PlayerCar>>,
    exploding_query: Query<&ExplosionTimer>,
) {
    for (entity, health) in player_query.iter() {
        if health.0 == 0 && exploding_query.get(entity).is_err() {
            commands.entity(entity).insert((
                ExplosionTimer(Timer::from_seconds(0.5, TimerMode::Once)),
                LinearVelocity::ZERO,
                AngularVelocity::ZERO,
            ));
            pending.0 = Some(GameState::PreGame);
            return;
        }
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
        .init_resource::<UiAction>()
        .init_resource::<PendingState>()
        .add_systems(Update, enter_pregame.run_if(in_state(GameState::Loading)))
        .add_systems(Update, check_game_state.run_if(in_state(GameState::Playing)))
        .add_systems(Update, handle_ui_actions)
        .add_systems(Update, apply_pending_state)
        .add_plugins((ai::AiPlugin, blaster::BlasterPlugin, car::CarPlugin, track::TrackPlugin, ui::UiPlugin));

    #[cfg(feature = "dev")]
    app.add_plugins(avian3d::debug_render::PhysicsDebugPlugin);

    app.run();
}