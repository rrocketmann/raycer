use bevy::prelude::*;
use bevy_light::DirectionalLightShadowMap;

use crate::car::{Car, CarCamera, CarVisual, PlayerCar, ARENA_RADIUS};

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

    let ground_size = 150.0;
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
    let mountain_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.30, 0.25),
        ..default()
    });

    let num_mountains = 24;
    for i in 0..num_mountains {
        let angle = (i as f32 / num_mountains as f32) * std::f32::consts::TAU;
        let variation = ((i * 7 + 3) % 5) as f32 * 0.6 + 0.8;
        let height = 6.0 * variation;
        let base_radius = 4.0 + variation * 1.5;

        let cone = meshes.add(
            Cone::default()
                .mesh()
                .resolution(5)
                .build(),
        );

        let dist = ARENA_RADIUS + base_radius * 0.3;

        commands.spawn((
            Mesh3d(cone),
            MeshMaterial3d(mountain_mat.clone()),
            Transform::from_xyz(
                angle.cos() * dist,
                0.0,
                angle.sin() * dist,
            )
            .with_scale(Vec3::new(base_radius * 2.0, height, base_radius * 2.0)),
        ));
    }
}