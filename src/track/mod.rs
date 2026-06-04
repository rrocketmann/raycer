use bevy::prelude::*;
use avian3d::prelude::*;
use bevy_light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap, ShadowFilteringMethod};

use crate::car::{Car, CarCamera, CarCollider, CarVisual, PlayerCar, CAR_DEFS, VehicleData};

#[derive(Component)]
struct MapRoot;

pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_world);
    }
}

fn spawn_world(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let def = &CAR_DEFS[0];
    let car_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(def.path));
    let car_root = commands.spawn((
        Car { speed: 0.0, yaw: 0.0 },
        PlayerCar,
        RigidBody::Dynamic,
        Position(Vec3::new(0.0, 3.0, 0.0)),
        Rotation::default(),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
    )).id();
    let half_height = def.collider.y * 0.5;
    commands.entity(car_root).insert((
        LinearDamping(0.5),
        AngularDamping(4.0),
        MaxLinearSpeed(50.0),
        MaxAngularSpeed(4.0),
        CenterOfMass(Vec3::new(0.0, -half_height, 0.0)),
        Friction::new(0.01),
        SweptCcd::NON_LINEAR,
        Mass(6.0),
        GravityScale(1.0),
        VehicleData::default(),
    ));

    commands.entity(car_root).with_children(|parent| {
        parent.spawn((
            Collider::cuboid(def.collider.x, def.collider.y, def.collider.z),
            Transform::from_translation(Vec3::new(0.0, half_height, 0.0)),
            CarCollider,
        ));
        parent.spawn((SceneRoot(car_scene), CarVisual));
    });

    let map_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset("Map.glb"));
    commands.spawn((
        SceneRoot(map_scene),
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(20.0, 60.0, 20.0)),
        RigidBody::Static,
        ColliderConstructorHierarchy::new(
            ColliderConstructor::TrimeshFromMeshWithConfig(TrimeshFlags::FIX_INTERNAL_EDGES),
        ),
        MapRoot,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            shadow_depth_bias: 0.02,
            shadow_normal_bias: 0.7,
            ..default()
        },
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.1,
            maximum_distance: 250.0,
            first_cascade_far_bound: 15.0,
            overlap_proportion: 0.3,
        }.build(),
    ));

    commands.insert_resource(DirectionalLightShadowMap { size: 4096 });

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 8.0, -15.0).looking_at(Vec3::ZERO, Vec3::Y),
        CarCamera,
        ShadowFilteringMethod::Gaussian,
    ));
}