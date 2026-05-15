use bevy::prelude::*;

use crate::car::{Car, CarCamera, PlayerCar};

pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_world)
            .add_systems(Startup, fix_camera_order.after(spawn_world))
            .add_systems(FixedUpdate, move_car);
    }
}

#[derive(Component)]
pub struct TrackPiece;

fn spawn_world(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let car = asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/raceCarRed.glb"));
    let road_straight = asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/roadStraight.glb"));
    let road_corner = asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/roadCornerSmall.glb"));
    let road_start = asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/roadStart.glb"));

    // Car - scaled up and positioned on the road
    commands.spawn((
        SceneRoot(car),
        Transform::from_xyz(5.0, 0.5, 5.0),
        Car { speed: 0.0, yaw: 0.0 },
        PlayerCar,
    ));

    // Build a simple oval track
    // Kenney road pieces are roughly 10 units long / 10 unit radius
    let piece_size = 10.0;
    let straights = 6;

    // Start/finish line at the beginning
    commands.spawn((
        SceneRoot(road_start.clone()),
        Transform::from_xyz(0.0, 0.0, 0.0),
        TrackPiece,
    ));

    // Straight pieces going north (positive Z)
    for i in 0..straights {
        commands.spawn((
            SceneRoot(road_straight.clone()),
            Transform::from_xyz(0.0, 0.0, (i as f32 + 1.0) * piece_size),
            TrackPiece,
        ));
    }

    // Top curve (turn right)
    let top_z = (straights as f32 + 1.0) * piece_size;
    commands.spawn((
        SceneRoot(road_corner.clone()),
        Transform::from_xyz(0.0, 0.0, top_z),
        TrackPiece,
    ));

    // Straight pieces going back (parallel, offset in X)
    let offset_x = piece_size;
    for i in 0..straights {
        let z = top_z - (i as f32 + 1.0) * piece_size;
        commands.spawn((
            SceneRoot(road_straight.clone()),
            Transform::from_xyz(offset_x, 0.0, z).with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
            TrackPiece,
        ));
    }

    // Bottom curve (turn back to start)
    commands.spawn((
        SceneRoot(road_corner.clone()),
        Transform::from_xyz(offset_x, 0.0, 0.0).with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        TrackPiece,
    ));

    // Ground plane
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(Plane3d::new(Vec3::Y, Vec2::splat(500.0))))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.22, 0.50, 0.22),
            ..default()
        })),
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            illuminance: 80000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.5, 0.0)),
    ));

    commands.spawn(AmbientLight {
        color: Color::srgb(0.9, 0.92, 1.0),
        brightness: 800.0,
        affects_lightmapped_meshes: true,
    });

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 10.0, -15.0).looking_at(Vec3::new(5.0, 0.0, 5.0), Vec3::Y),
        CarCamera,
    ));
}

fn move_car(
    mut car_query: Query<(&Car, &mut Transform), With<PlayerCar>>,
) {
    for (car, mut transform) in car_query.iter_mut() {
        let forward = Vec3::new(car.yaw.sin(), 0.0, car.yaw.cos());
        transform.translation += forward * car.speed * 0.016;
        transform.rotation = Quat::from_rotation_y(car.yaw);
    }
}

fn fix_camera_order(mut query: Query<&mut Camera, With<CarCamera>>) {
    for mut cam in query.iter_mut() {
        cam.order = 0;
    }
}