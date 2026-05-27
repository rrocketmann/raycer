use bevy::prelude::*;
use avian3d::prelude::*;
use bevy_light::{DirectionalLightShadowMap, GlobalAmbientLight, ShadowFilteringMethod};

use crate::car::{Car, CarCamera, CarVisual, PlayerCar};

pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_world);
    }
}

const GROUND_HALF_SIZE: f32 = 500.0;
const GROUND_THICKNESS: f32 = 0.5;

fn spawn_world(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let car_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/raceCarRed.glb"));
    let car_root = commands.spawn((
        Transform::from_xyz(0.0, 3.0, 0.0),
        Car { speed: 0.0, yaw: 0.0 },
        PlayerCar,
        CarVisual,
        RigidBody::Dynamic,
        Position(Vec3::new(0.0, 3.0, 0.0)),
        Rotation::default(),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
    )).id();
    commands.entity(car_root).insert((
        LinearDamping(1.0),
        AngularDamping(3.0),
        MaxLinearSpeed(30.0),
        MaxAngularSpeed(2.0),
        CenterOfMass(Vec3::new(0.0, -0.05, 0.0)),
        SweptCcd::NON_LINEAR,
        Mass(15.0),
    ));

    commands.entity(car_root).with_children(|parent| {
        parent.spawn((
            SceneRoot(car_scene),
            Transform::from_xyz(0.0, -0.42, 0.0),
        ));

        for (lx, lz) in [(0.4_f32, 0.8_f32), (-0.4, 0.8), (0.4, -0.8), (-0.4, -0.8)] {
            parent.spawn((
                Collider::cylinder(0.3, 0.25),
                Friction::new(1.5),
                Transform::from_xyz(lx, -0.05, lz)
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            ));
        }
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(
            GROUND_HALF_SIZE * 2.0,
            GROUND_THICKNESS,
            GROUND_HALF_SIZE * 2.0,
        ))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.28, 0.25),
            perceptual_roughness: 0.95,
            ..default()
        })),
        Transform::from_xyz(0.0, -GROUND_THICKNESS / 2.0, 0.0),
        RigidBody::Static,
        Collider::cuboid(GROUND_HALF_SIZE, GROUND_THICKNESS / 2.0, GROUND_HALF_SIZE),
        Friction::new(1.5),
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
        ShadowFilteringMethod::Gaussian,
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.95, 0.92, 0.85),
        brightness: 0.15,
        affects_lightmapped_meshes: true,
    });
}
