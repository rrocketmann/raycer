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
pub struct CarRoof;

fn spawn_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(Plane3d::new(Vec3::Y, Vec2::splat(500.0))))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.55, 0.25),
            ..default()
        })),
    ));

    // Grid lines
    for i in -250..=250i32 {
        if i % 10 != 0 {
            continue;
        }
        let thick = if i % 50 == 0 { 0.2 } else { 0.05 };
        let col = if i % 50 == 0 {
            Color::srgb(0.18, 0.4, 0.18)
        } else {
            Color::srgb(0.22, 0.48, 0.22)
        };
        commands.spawn((
            Mesh3d(meshes.add(Mesh::from(Cuboid::new(500.0, thick, thick)))),
            MeshMaterial3d(materials.add(StandardMaterial { base_color: col, ..default() })),
            Transform::from_xyz(0.0, 0.01, i as f32),
        ));
        commands.spawn((
            Mesh3d(meshes.add(Mesh::from(Cuboid::new(thick, thick, 500.0)))),
            MeshMaterial3d(materials.add(StandardMaterial { base_color: col, ..default() })),
            Transform::from_xyz(i as f32, 0.01, 0.0),
        ));
    }

    // Car body with child roof
    let car_mesh = meshes.add(Mesh::from(Cuboid::new(1.8, 0.8, 3.6)));
    let car_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.15, 0.15),
        ..default()
    });
    let roof_mesh = meshes.add(Mesh::from(Cuboid::new(1.4, 0.5, 1.5)));
    let roof_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.1, 0.1),
        ..default()
    });

    commands.spawn((
        Mesh3d(car_mesh),
        MeshMaterial3d(car_mat),
        Transform::from_xyz(0.0, 0.5, 0.0),
        Car { speed: 0.0, yaw: 0.0 },
        PlayerCar,
        Children::default(),
    )).with_children(|parent| {
        parent.spawn((
            Mesh3d(roof_mesh),
            MeshMaterial3d(roof_mat),
            Transform::from_xyz(0.0, 0.65, -0.3),
            CarRoof,
        ));
    });

    // Directional light (sun)
    commands.spawn((
        DirectionalLight {
            illuminance: 50000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.6, 0.4, 0.0)),
    ));

    // Ambient light
    commands.spawn(AmbientLight {
        color: Color::srgb(0.8, 0.85, 1.0),
        brightness: 500.0,
        affects_lightmapped_meshes: true,
    });

    // 3D camera (order 1 = renders after 2D UI camera at order 0)
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, -10.0).looking_at(Vec3::ZERO, Vec3::Y),
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