use bevy::prelude::*;

use crate::car::{Car, CarCamera, CarVisual, PlayerCar, MAP_HALF_SIZE};

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
        Car { speed: 0.0, yaw: 0.0 },
        PlayerCar,
        CarVisual,
    ));

    let ground_size = 500.0;
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(Plane3d::new(Vec3::Y, Vec2::splat(ground_size))))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.76, 0.70, 0.50),
            ..default()
        })),
        GroundPlane,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 16000.0,
            shadows_enabled: true,
            shadow_depth_bias: 0.02,
            shadow_normal_bias: 0.6,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.2, 0.4, 0.0)),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 8.0, -15.0).looking_at(Vec3::ZERO, Vec3::Y),
        CarCamera,
    ));

    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.95, 0.92, 0.85),
        brightness: 120.0,
        affects_lightmapped_meshes: true,
    });

    spawn_barrier(&mut commands, &mut meshes, &mut materials);
}

fn spawn_barrier(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let barrier_radius = MAP_HALF_SIZE + 2.0;
    let wall_height = 2.0;

    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.33, 0.30),
        cull_mode: None,
        ..default()
    });

    let cylinder = meshes.add(
        Cylinder::new(barrier_radius, wall_height)
            .mesh()
            .resolution(512)
            .without_caps()
            .build(),
    );

    commands.spawn((
        Mesh3d(cylinder),
        MeshMaterial3d(wall_mat),
        Transform::from_xyz(0.0, wall_height * 0.5, 0.0),
    ));
}