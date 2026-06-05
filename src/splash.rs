use bevy::prelude::*;
use bevy::asset::LoadedUntypedAsset;

#[derive(Component)]
struct LoadingScreen;

#[derive(Resource)]
struct LoadingOverlay {
    handles: Vec<Handle<LoadedUntypedAsset>>,
}

fn start_loading(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut started: Local<bool>,
) {
    if *started {
        return;
    }
    *started = true;

    let paths = [
        "models/race.glb",
        "models/blaster-a.glb",
        "Map.glb",
    ];

    let handles: Vec<Handle<LoadedUntypedAsset>> = paths.iter().map(|&p| asset_server.load_untyped(p)).collect();

    commands.spawn((Camera2d, Camera { order: 10, ..default() }, LoadingScreen));
    commands.spawn((
        Sprite::from_color(Color::srgba(0.08, 0.08, 0.08, 1.0), Vec2::new(10000.0, 10000.0)),
        Transform::from_translation(Vec3::new(0.0, 0.0, -1000.0)),
        LoadingScreen,
    ));
    commands.spawn((
        Text2d::new("Loading..."),
        TextFont { font_size: 48.0, ..default() },
        TextColor(Color::WHITE),
        LoadingScreen,
    ));

    commands.insert_resource(LoadingOverlay { handles });
}

fn finish_loading(
    handles: Option<Res<LoadingOverlay>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    q: Query<Entity, With<LoadingScreen>>,
) {
    let Some(h) = handles else { return };
    for handle in &h.handles {
        if !asset_server.is_loaded_with_dependencies(handle) {
            return;
        }
    }

    for entity in q.iter() {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<LoadingOverlay>();
}

pub struct SplashPlugin;

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (start_loading, finish_loading));
    }
}
