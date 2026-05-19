use bevy::prelude::*;
use bevy_light::{DirectionalLightShadowMap, GlobalAmbientLight};

use crate::car::{Car, CarCamera, CarVisual, PlayerCar};

pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_world);
    }
}

#[derive(Component)]
pub struct GroundPlane;

fn spawn_world(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let car_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/raceCarRed.glb"));
    commands.spawn((
        SceneRoot(car_scene),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Car { speed: 0.0, yaw: 0.0, y_velocity: 0.0, airborne: false },
        PlayerCar,
        CarVisual,
    ));

    let map_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset("Map.glb"));
    commands.spawn((
        SceneRoot(map_scene),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 16000.0,
            shadows_enabled: true,
            shadow_depth_bias: 0.02,
            shadow_normal_bias: 0.2,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.2, 0.4, 0.0)),
    ));

    commands.insert_resource(DirectionalLightShadowMap { size: 4096 });

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 8.0, -15.0).looking_at(Vec3::ZERO, Vec3::Y),
        CarCamera,
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.95, 0.92, 0.85),
        brightness: 120.0,
        affects_lightmapped_meshes: true,
    });
}