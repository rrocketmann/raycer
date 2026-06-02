use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin, MeshMaterial3d, StandardMaterial};
use bevy::prelude::*;
use avian3d::prelude::*;
use bevy_light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap, GlobalAmbientLight, ShadowFilteringMethod};
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

use crate::car::{Car, CarCamera, PlayerCar, VehicleData};

const CAR_COLLIDER_SIZE: Vec3 = Vec3::new(0.73, 0.40, 1.35);
const CAR_COLLIDER_OFFSET: Vec3 = Vec3::new(-0.365, 0.20, -0.675);

#[derive(Component)]
struct MapRoot;

#[derive(Component)]
struct TrackMaterialApplied;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
struct TrackExt {
    #[uniform(100)]
    variation: Vec4,
}

impl MaterialExtension for TrackExt {
    fn fragment_shader() -> ShaderRef {
        "shaders/track_material.wgsl".into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/track_material.wgsl".into()
    }
}

type TrackMaterial = ExtendedMaterial<StandardMaterial, TrackExt>;

pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TrackMaterial>::default())
            .add_systems(Startup, spawn_world)
            .add_systems(Update, replace_map_materials);
    }
}

fn spawn_world(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let car_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/raceCarRed.glb"));
    let car_root = commands.spawn((
        Car { speed: 0.0, yaw: 0.0 },
        PlayerCar,
        RigidBody::Dynamic,
        Position(Vec3::new(0.0, 3.0, 0.0)),
        Rotation::default(),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
    )).id();
    commands.entity(car_root).insert((
        LinearDamping(0.5),
        AngularDamping(4.0),
        MaxLinearSpeed(50.0),
        MaxAngularSpeed(4.0),
        CenterOfMass(Vec3::new(-0.365, 0.0, -0.675)),
        Friction::new(0.01),
        SweptCcd::NON_LINEAR,
        Mass(15.0),
        GravityScale(1.0),
        VehicleData::default(),
    ));

    commands.entity(car_root).with_children(|parent| {
        parent.spawn((
            Collider::cuboid(CAR_COLLIDER_SIZE.x, CAR_COLLIDER_SIZE.y, CAR_COLLIDER_SIZE.z),
            Transform::from_translation(CAR_COLLIDER_OFFSET),
        ));
    });

    commands.entity(car_root).with_children(|parent| {
        parent.spawn(SceneRoot(car_scene));
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
            illuminance: 4000.0,
            shadows_enabled: true,
            shadow_depth_bias: 0.02,
            shadow_normal_bias: 0.4,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.2, 0.4, 0.0)),
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.1,
            maximum_distance: 80.0,
            first_cascade_far_bound: 5.0,
            overlap_proportion: 0.2,
        }.build(),
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
        brightness: 0.3,
        affects_lightmapped_meshes: true,
    });
}

fn replace_map_materials(
    map_query: Query<Entity, With<MapRoot>>,
    children_query: Query<&Children>,
    standard_materials: Res<Assets<StandardMaterial>>,
    mut track_materials: ResMut<Assets<TrackMaterial>>,
    mut commands: Commands,
    material_query: Query<(Entity, &MeshMaterial3d<StandardMaterial>), Without<TrackMaterialApplied>>,
) {
    for map_entity in &map_query {
        for child in children_query.iter_descendants(map_entity) {
            if let Ok((entity, std_mat_handle)) = material_query.get(child) {
                if let Some(std_mat) = standard_materials.get(&std_mat_handle.0) {
                    let track_mat = TrackMaterial {
                        base: std_mat.clone(),
                        extension: TrackExt { variation: Vec4::new(0.3, 0.25, 0.2, 0.0) },
                    };
                    let track_handle = track_materials.add(track_mat);
                    commands.entity(entity)
                        .remove::<MeshMaterial3d<StandardMaterial>>()
                        .insert(MeshMaterial3d(track_handle))
                        .insert(TrackMaterialApplied);
                }
            }
        }
    }
}