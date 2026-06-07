use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::HashSet;
use crate::blaster::{Bullet, ExcludeMeshRayCast, BLASTER_DEFS, BULLET_SPEED, BULLET_RADIUS};
use crate::car::{AiCar, CarVisual, PlayerCar, CAR_DEFS, mount_y};

#[derive(Component)]
pub struct AiBlasterVisual;

#[derive(Component)]
struct AiComputePivot;

#[derive(Resource, Default)]
struct AiPivotCache {
    pivot: Option<Vec3>,
}

#[derive(Resource)]
struct AiShootTimer {
    timer: Timer,
}

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AiPivotCache>()
            .insert_resource(AiShootTimer {
                timer: Timer::from_seconds(1.5, TimerMode::Repeating),
            })
            .add_systems(Startup, spawn_ai_car)
            .add_systems(Update, (
                ai_compute_pivot,
                ai_aim_blaster,
            ).chain())
            .add_systems(Update, ai_drive)
            .add_systems(Update, ai_shoot);
    }
}

const AI_CAR_INDEX: usize = 3;
const AI_BLASTER_INDEX: usize = 1;
const AI_SPAWN_POS: Vec3 = Vec3::new(10.0, 3.0, 10.0);

fn spawn_ai_car(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let def = &CAR_DEFS[AI_CAR_INDEX];
    let car_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(def.path));
    let blaster_def = &BLASTER_DEFS[AI_BLASTER_INDEX];
    let blaster_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(blaster_def.path));
    let half_height = def.collider.y * 0.5;
    let mount = Vec3::new(0.0, mount_y(def.collider.y), 0.0);

    let ai_root = commands.spawn((
        AiCar,
        RigidBody::Dynamic,
        Position(AI_SPAWN_POS),
        Rotation::default(),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
        LinearDamping(0.5),
        AngularDamping(1.0),
        MaxLinearSpeed(50.0),
        MaxAngularSpeed(4.0),
        CenterOfMass(Vec3::ZERO),
        Friction::new(0.01),
        SweptCcd::NON_LINEAR,
        Mass(6.0),
        GravityScale(1.0),
    )).id();

    commands.entity(ai_root).with_children(|parent| {
        parent.spawn((
            Collider::cuboid(def.collider.x, def.collider.y, def.collider.z),
            Transform::from_translation(Vec3::new(0.0, half_height, 0.0)),
            CollisionLayers::new(LayerMask(0b010), LayerMask(0xFFFFFFFF)),
        ));
        parent.spawn((
            SceneRoot(car_scene),
            CarVisual,
        ));
        parent.spawn((
            SceneRoot(blaster_scene),
            Transform::from_translation(mount).with_scale(Vec3::splat(blaster_def.scale)),
            AiBlasterVisual,
            AiComputePivot,
        ));
    });
}

fn ai_compute_pivot(
    mut commands: Commands,
    mut pivot_cache: ResMut<AiPivotCache>,
    ai_query: Query<&GlobalTransform, With<AiCar>>,
    blaster_query: Query<(Entity, &GlobalTransform, &Transform), (With<AiBlasterVisual>, With<AiComputePivot>)>,
    children_query: Query<&Children>,
    mesh_query: Query<&GlobalTransform, With<Mesh3d>>,
) {
    for (entity, _blaster_global, blaster_transform) in blaster_query.iter() {
        let mut center_world = Vec3::ZERO;
        let mut count = 0;

        for desc in children_query.iter_descendants(entity) {
            if let Ok(desc_global) = mesh_query.get(desc) {
                center_world += desc_global.translation();
                count += 1;
            }
        }

        if count == 0 { continue; }
        center_world /= count as f32;

        let Ok(ai_global) = ai_query.single() else { continue };
        let center_local = ai_global.affine().inverse().transform_point(center_world);

        let def = &CAR_DEFS[AI_CAR_INDEX];
        let mount = Vec3::new(0.0, mount_y(def.collider.y), 0.0);
        let rot = blaster_transform.rotation;
        let s = blaster_transform.scale.x;

        let pivot = rot.inverse() * (center_local - mount) / s;

        pivot_cache.pivot = Some(pivot);
        commands.entity(entity).remove::<AiComputePivot>();
        return;
    }
}

fn ai_aim_blaster(
    ai_query: Query<&GlobalTransform, With<AiCar>>,
    player_query: Query<&GlobalTransform, (With<PlayerCar>, Without<AiCar>)>,
    pivot_cache: Res<AiPivotCache>,
    mut blaster_query: Query<&mut Transform, (With<AiBlasterVisual>, Without<PlayerCar>)>,
) {
    let Ok(ai_global) = ai_query.single() else { return };
    let Ok(player_global) = player_query.single() else { return };
    let Ok(mut blaster) = blaster_query.single_mut() else { return };

    let def = &CAR_DEFS[AI_CAR_INDEX];
    let mount = Vec3::new(0.0, mount_y(def.collider.y), 0.0);
    let s = blaster.scale.x;
    let pivot = pivot_cache.pivot.unwrap_or(Vec3::ZERO);

    let ai_pos = ai_global.translation();
    let ai_rot = ai_global.rotation();
    let player_pos = player_global.translation();
    let aim_point = player_pos + Vec3::new(0.0, 1.0, 0.0);

    let blaster_world_mount = ai_pos + ai_rot * mount;
    let local_aim = ai_rot.inverse() * (aim_point - blaster_world_mount);
    if local_aim.length_squared() < 0.01 { return; }
    let local_dir = local_aim.normalize();
    let yaw = f32::atan2(-local_dir.x, -local_dir.z);
    let horiz_len = Vec2::new(local_dir.x, local_dir.z).length();
    let pitch = f32::atan2(local_dir.y, horiz_len);

    let rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    blaster.translation = mount - rotation * (s * pivot);
    blaster.rotation = rotation;
}

fn ai_drive(
    time: Res<Time>,
    mut ai_query: Query<(&GlobalTransform, &Rotation, &mut LinearVelocity, &mut AngularVelocity), With<AiCar>>,
    player_query: Query<&GlobalTransform, (With<PlayerCar>, Without<AiCar>)>,
) {
    let Ok((ai_transform, ai_rot, mut lin_vel, mut ang_vel)) = ai_query.single_mut() else { return };
    let Ok(player_transform) = player_query.single() else { return };

    let ai_pos = ai_transform.translation();
    let player_pos = player_transform.translation();

    let to_player = player_pos - ai_pos;
    let flat_to_player = Vec3::new(to_player.x, 0.0, to_player.z);

    if flat_to_player.length_squared() < 1.0 { return; }

    let desired_dir = flat_to_player.normalize();
    let ai_forward = ai_rot.0 * Vec3::Z;
    let ai_flat_forward = Vec3::new(ai_forward.x, 0.0, ai_forward.z).normalize_or(Vec3::Z);

    let cross = ai_flat_forward.cross(Vec3::Y);
    let dot = cross.dot(desired_dir);
    let steer = dot.clamp(-1.0, 1.0);

    ang_vel.0 = Vec3::Y * steer * 3.0;

    let speed = lin_vel.0.length();
    let dt = time.delta_secs();
    let vel = lin_vel.0;
    if speed < 25.0 {
        lin_vel.0 += ai_flat_forward * 2400.0 * dt;
    }
    lin_vel.0 -= vel.normalize_or(Vec3::ZERO) * 3.0 * dt;
    lin_vel.0 -= vel * speed * 0.4 * dt;
}

fn ai_shoot(
    mut timer: ResMut<AiShootTimer>,
    time: Res<Time>,
    ai_blaster_query: Query<&GlobalTransform, With<AiBlasterVisual>>,
    ai_query: Query<Entity, With<AiCar>>,
    children_query: Query<&Children>,
    player_query: Query<&GlobalTransform, (With<PlayerCar>, Without<AiCar>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    timer.timer.tick(time.delta());
    if !timer.timer.just_finished() { return; }

    let Ok(blaster_global) = ai_blaster_query.single() else { return };
    let Ok(ai_entity) = ai_query.single() else { return };
    let Ok(player_global) = player_query.single() else { return; };

    let blaster_pos = blaster_global.translation();
    let aim_point = player_global.translation() + Vec3::new(0.0, 1.0, 0.0);
    let direction = (aim_point - blaster_pos).normalize_or(Vec3::Z);
    let spawn_pos = blaster_pos + direction * 1.0;

    let mut exclude = HashSet::new();
    exclude.insert(ai_entity);
    for desc in children_query.iter_descendants(ai_entity) {
        exclude.insert(desc);
    }

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(BULLET_RADIUS).mesh().ico(2).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Srgba::hex("ff4400").unwrap().into(),
            emissive: LinearRgba::new(6.0, 1.0, 0.0, 1.0),
            ..default()
        })),
        Transform::from_translation(spawn_pos),
        Bullet {
            velocity: direction * BULLET_SPEED,
            lifetime: Timer::from_seconds(5.0, TimerMode::Once),
        },
        ExcludeMeshRayCast(exclude),
    ));
}