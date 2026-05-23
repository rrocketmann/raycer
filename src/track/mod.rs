use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureFormat, TextureUsages};
use bevy_camera::RenderTarget;
use avian3d::prelude::*;
use bevy_light::{DirectionalLightShadowMap, GlobalAmbientLight};

use crate::car::{Car, CarCamera, CarVisual, MinimapCamera, PlayerCar};

pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_world)
            .add_systems(Update, update_minimap_camera);
    }
}

#[derive(Resource)]
pub struct MinimapImage(pub Handle<Image>);

fn create_minimap_image(width: u32, height: u32) -> Image {
    let size = Extent3d { width, height, depth_or_array_layers: 1 };
    let mut image = Image {
        texture_descriptor: bevy::render::render_resource::TextureDescriptor {
            label: Some("minimap_image"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: bevy::render::render_resource::TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        },
        ..default()
    };
    image.resize(size);
    image
}

fn spawn_world(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
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
        Friction::new(1.5),
        SweptCcd::NON_LINEAR,
        Mass(15.0),
        Collider::cuboid(0.8, 0.25, 1.6),
    ));

    commands.entity(car_root).with_children(|parent| {
        parent.spawn((
            SceneRoot(car_scene),
            Transform::from_xyz(0.0, -0.09, 0.0),
        ));
    });

    let map_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset("Map.glb"));
    commands.spawn((
        SceneRoot(map_scene),
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(50.0)),
        RigidBody::Static,
        ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
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

    let minimap_image = images.add(create_minimap_image(256, 256));
    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(minimap_image.clone().into()),
        Transform::from_xyz(0.0, 40.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        MinimapCamera,
    ));
    commands.insert_resource(MinimapImage(minimap_image));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.95, 0.92, 0.85),
        brightness: 120.0,
        affects_lightmapped_meshes: true,
    });
}

fn update_minimap_camera(
    car_query: Query<(&Car, &Position), With<PlayerCar>>,
    mut minimap_cam: Query<&mut Transform, (With<MinimapCamera>, Without<PlayerCar>)>,
) {
    let Ok((car, car_pos)) = car_query.single() else { return };
    let up = Vec3::new(car.yaw.sin(), 0.0, car.yaw.cos());
    for mut cam in minimap_cam.iter_mut() {
        cam.translation.x = car_pos.0.x;
        cam.translation.z = car_pos.0.z;
        cam.translation.y = 40.0;
        cam.look_at(car_pos.0, up);
    }
}
