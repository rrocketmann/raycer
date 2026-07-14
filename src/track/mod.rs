use bevy::prelude::*;
use avian3d::prelude::*;
use bevy_light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap, ShadowFilteringMethod};
use crate::car::{Car, CarCamera, CarCollider, CarVisual, PlayerCar, CAR_DEFS, VehicleData, CarSelection, Health, spawn_health_indicators};
use crate::blaster::{BlasterSelection, BLASTER_DEFS};
use crate::GameState;
use crate::MaxHealthPoints;

#[derive(Component)]
struct MapRoot;

#[derive(Component)]
struct WorldMarker;

pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::PreGame), spawn_world)
            .add_systems(OnExit(GameState::PreGame), cleanup_world)
            .add_systems(OnEnter(GameState::Playing), spawn_world)
            .add_systems(OnExit(GameState::Eliminated), cleanup_world);
    }
}

fn spawn_world(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    car_selection: Res<CarSelection>,
    blaster_selection: Res<BlasterSelection>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    max_hp: Res<MaxHealthPoints>,
) {
    let def = &CAR_DEFS[car_selection.index];
    let car_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(def.path));
    let blaster_def = &BLASTER_DEFS[blaster_selection.index];
    let blaster_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(blaster_def.path));
    let mount_y = crate::car::mount_y(def.collider.y);
    let car_root = commands.spawn((
        Car { speed: 0.0, yaw: 0.0 },
        PlayerCar,
        WorldMarker,
        RigidBody::Dynamic,
        Position(Vec3::new(0.0, 3.0, 0.0)),
        Rotation::default(),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
    )).insert(Health(max_hp.0)).id();
    spawn_health_indicators(car_root, &mut commands, &mut meshes, &mut materials, def.collider.y, max_hp.0);
    let half_height = def.collider.y * 0.5;
    commands.entity(car_root).insert((
        LinearDamping(0.5),
        AngularDamping(1.0),
        MaxLinearSpeed(50.0),
        MaxAngularSpeed(4.0),
        CenterOfMass(Vec3::ZERO),
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
            CollisionLayers::new(LayerMask(0b010), LayerMask(0xFFFFFFFF)),
            CarCollider,
        ));
        parent.spawn((SceneRoot(car_scene), CarVisual));
        parent.spawn((
            SceneRoot(blaster_scene),
            Transform::from_translation(Vec3::new(0.0, mount_y, 0.0))
                .with_scale(Vec3::splat(blaster_def.scale))
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
            crate::blaster::BlasterVisual,
            crate::blaster::ComputePivot,
        ));
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
        WorldMarker,
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
        WorldMarker,
    ));

    commands.insert_resource(DirectionalLightShadowMap { size: 4096 });

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 1.2,
            ..default()
        }),
        Transform::from_xyz(0.0, 8.0, -22.0).looking_at(Vec3::ZERO, Vec3::Y),
        CarCamera,
        ShadowFilteringMethod::Gaussian,
        WorldMarker,
    ));
}

fn cleanup_world(mut commands: Commands, q: Query<Entity, With<WorldMarker>>) {
    for e in q.iter() {
        commands.entity(e).despawn();
    }
}