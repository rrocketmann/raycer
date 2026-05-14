use bevy::prelude::*;
use crate::car::PlayerCar;
use serde::{Deserialize, Serialize};

pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_default_track);
    }
}

#[derive(Component)]
pub struct Track {
    pub name: String,
    pub checkpoints: Vec<Vec3>,
    pub width: f32,
}

#[derive(Component)]
pub struct Checkpoint {
    pub index: usize,
    pub is_finish: bool,
}

#[derive(Serialize, Deserialize)]
pub struct TrackData {
    pub name: String,
    pub checkpoints: Vec<[f32; 3]>,
    pub width: f32,
}

pub fn spawn_default_track(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create a simple oval track
    let track_points = generate_oval_track(50.0, 30.0, 32);
    let track_width = 12.0;

    let track_mesh = create_track_mesh(&track_points, track_width);
    let track_material = StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.35),
        ..default()
    };

    commands.spawn((
        Mesh3d(meshes.add(track_mesh)),
        MeshMaterial3d(materials.add(track_material)),
        Track {
            name: "Default Oval".to_string(),
            checkpoints: track_points.clone(),
            width: track_width,
        },
    ));

    // Spawn checkpoint markers
    for (i, point) in track_points.iter().enumerate().step_by(4) {
        commands.spawn((
            Mesh3d(meshes.add(Mesh::from(Cuboid::new(1.0, 2.0, track_width)))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: if i == 0 {
                    Color::srgb(1.0, 0.0, 0.0)
                } else {
                    Color::srgb(0.0, 1.0, 0.0)
                },
                ..default()
            })),
            Transform::from_xyz(point.x, 1.0, point.z),
            Checkpoint {
                index: i / 4,
                is_finish: i == 0,
            },
        ));
    }

    // Ground plane
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(Plane3d::new(Vec3::Y, Vec2::splat(100.0))))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.5, 0.2),
            ..default()
        })),
    ));

    // Light
    commands.spawn((
        PointLight {
            intensity: 1_000_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 20.0, 0.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 15.0, -20.0).looking_at(Vec3::ZERO, Vec3::Y),
        PlayerCar,
    ));
}

fn generate_oval_track(radius_x: f32, radius_z: f32, segments: usize) -> Vec<Vec3> {
    (0..segments)
        .map(|i| {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            Vec3::new(
                angle.sin() * radius_x,
                0.0,
                angle.cos() * radius_z,
            )
        })
        .collect()
}

fn create_track_mesh(points: &[Vec3], width: f32) -> Mesh {
    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for i in 0..points.len() {
        let current = points[i];
        let next = points[(i + 1) % points.len()];
        let dir = (next - current).normalize();
        let right = Vec3::new(-dir.z, 0.0, dir.x);

        let left = current - right * width / 2.0;
        let right = current + right * width / 2.0;

        let idx = positions.len() as u32;
        positions.push([left.x, left.y, left.z]);
        positions.push([right.x, right.y, right.z]);
        uvs.push([0.0, i as f32]);
        uvs.push([1.0, i as f32]);

        if i < points.len() - 1 {
            indices.push(idx);
            indices.push(idx + 1);
            indices.push(idx + 2);
            indices.push(idx + 1);
            indices.push(idx + 3);
            indices.push(idx + 2);
        }
    }

    // Close the loop
    let last_idx = (positions.len() - 2) as u32;
    indices.push(last_idx);
    indices.push(last_idx + 1);
    indices.push(0);
    indices.push(last_idx + 1);
    indices.push(1);
    indices.push(0);

    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));

    mesh
}
