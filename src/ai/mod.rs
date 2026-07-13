use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::HashSet;
use std::time::Duration;
use crate::blaster::{Bullet, ExcludeMeshRayCast, BLASTER_DEFS, BULLET_RADIUS, BULLET_SPEED};
use crate::car::{AiCar, CarVisual, PlayerCar, CAR_DEFS, mount_y};
use crate::GameState;
use crate::AiEnemyCount;
use rand::Rng;

#[derive(Component)]
pub struct AiBlasterVisual;

#[derive(Component)]
struct AiComputePivot;

#[derive(Component, Default)]
struct AiPivotCache {
    pivot: Option<Vec3>,
}

#[derive(Component)]
struct AiShootTimer {
    timer: Timer,
}

#[derive(Component)]
struct AiConfig {
    car_index: usize,
    bullet_color: Srgba,
    bullet_emissive: LinearRgba,
}

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::PreGame), spawn_ai_cars)
            .add_systems(OnExit(GameState::PreGame), cleanup_ai_cars)
            .add_systems(OnEnter(GameState::Playing), spawn_ai_cars)
            .add_systems(OnExit(GameState::Playing), cleanup_ai_cars)
            .add_systems(Update, sync_ai_count.run_if(in_state(GameState::PreGame)))
            .add_systems(Update, (
                ai_compute_pivot,
                ai_aim_blaster,
            ).chain())
            .add_systems(Update, ai_drive.run_if(in_state(GameState::Playing)))
            .add_systems(Update, ai_shoot.run_if(in_state(GameState::Playing)));
    }
}

#[derive(Component)]
struct AiSpawnMarker;

fn spawn_ai_cars(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    enemy_count: Res<AiEnemyCount>,
) {
    let car_options: Vec<usize> = vec![3, 5, 8, 10, 13, 15, 0, 6];
    let blaster_options: Vec<usize> = vec![1, 3, 5, 7, 9, 11, 13, 15, 17, 0];
    let bullet_colors: Vec<(Srgba, LinearRgba)> = vec![
        (Srgba::hex("ff4400").unwrap(), LinearRgba::new(6.0, 1.0, 0.0, 1.0)),
        (Srgba::hex("00bbff").unwrap(), LinearRgba::new(0.0, 4.0, 6.0, 1.0)),
        (Srgba::hex("cc00ff").unwrap(), LinearRgba::new(6.0, 0.0, 4.0, 1.0)),
        (Srgba::hex("00ff88").unwrap(), LinearRgba::new(0.0, 6.0, 2.0, 1.0)),
        (Srgba::hex("ffaa00").unwrap(), LinearRgba::new(6.0, 4.0, 0.0, 1.0)),
        (Srgba::hex("ff0088").unwrap(), LinearRgba::new(6.0, 0.0, 3.0, 1.0)),
        (Srgba::hex("88ff00").unwrap(), LinearRgba::new(3.0, 6.0, 0.0, 1.0)),
        (Srgba::hex("0088ff").unwrap(), LinearRgba::new(0.0, 3.0, 6.0, 1.0)),
        (Srgba::hex("ff6600").unwrap(), LinearRgba::new(6.0, 2.0, 0.0, 1.0)),
        (Srgba::hex("aa00ff").unwrap(), LinearRgba::new(4.0, 0.0, 6.0, 1.0)),
    ];

    let count = enemy_count.0;
    for i in 0..count {
        let car_index = car_options[i % car_options.len()];
        let blaster_index = blaster_options[i % blaster_options.len()];
        let (bullet_color, bullet_emissive) = bullet_colors[i % bullet_colors.len()];

        let angle = i as f32 * std::f32::consts::TAU / count as f32;
        let radius = 40.0;
        let pos = Vec3::new(angle.cos() * radius, 3.0, angle.sin() * radius);
        let def = &CAR_DEFS[car_index];
        let car_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(def.path));
        let blaster_def = &BLASTER_DEFS[blaster_index];
        let blaster_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(blaster_def.path));
        let half_height = def.collider.y * 0.5;
        let mount = Vec3::new(0.0, mount_y(def.collider.y), 0.0);

        let ai_root = commands.spawn((
            AiCar,
            AiSpawnMarker,
            RigidBody::Dynamic,
            Position(pos),
            Rotation(Quat::from_rotation_y(f32::atan2(-pos.x, -pos.z))),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
            LinearDamping(0.5),
            AngularDamping(1.0),
            MaxLinearSpeed(80.0),
            MaxAngularSpeed(4.0),
            CenterOfMass(Vec3::ZERO),
            Friction::new(0.01),
            SweptCcd::NON_LINEAR,
            Mass(6.0),
        )).id();

        let mut rng = rand::rng();
        let shoot_interval = rng.random_range(1.0..4.0);
        commands.entity(ai_root).insert((
            GravityScale(1.0),
            AiShootTimer { timer: Timer::from_seconds(shoot_interval, TimerMode::Repeating) },
            AiConfig { car_index, bullet_color, bullet_emissive },
        ));

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
                Transform::from_translation(mount)
                    .with_scale(Vec3::splat(blaster_def.scale))
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                AiBlasterVisual,
                AiComputePivot,
                AiPivotCache::default(),
            ));
        });
    }
}

fn cleanup_ai_cars(mut commands: Commands, q: Query<Entity, With<AiSpawnMarker>>) {
    for e in q.iter() {
        commands.entity(e).despawn();
    }
}

fn sync_ai_count(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    enemy_count: Res<AiEnemyCount>,
    ai_query: Query<Entity, With<AiSpawnMarker>>,
) {
    let desired = enemy_count.0;
    let current = ai_query.iter().count();

    if current > desired {
        let to_remove: Vec<Entity> = ai_query.iter().skip(desired).collect();
        for e in to_remove {
            commands.entity(e).despawn();
        }
    } else if current < desired {
        let car_options: Vec<usize> = vec![3, 5, 8, 10, 13, 15, 0, 6];
        let blaster_options: Vec<usize> = vec![1, 3, 5, 7, 9, 11, 13, 15, 17, 0];
        let bullet_colors: Vec<(Srgba, LinearRgba)> = vec![
            (Srgba::hex("ff4400").unwrap(), LinearRgba::new(6.0, 1.0, 0.0, 1.0)),
            (Srgba::hex("00bbff").unwrap(), LinearRgba::new(0.0, 4.0, 6.0, 1.0)),
            (Srgba::hex("cc00ff").unwrap(), LinearRgba::new(6.0, 0.0, 4.0, 1.0)),
            (Srgba::hex("00ff88").unwrap(), LinearRgba::new(0.0, 6.0, 2.0, 1.0)),
            (Srgba::hex("ffaa00").unwrap(), LinearRgba::new(6.0, 4.0, 0.0, 1.0)),
            (Srgba::hex("ff0088").unwrap(), LinearRgba::new(6.0, 0.0, 3.0, 1.0)),
            (Srgba::hex("88ff00").unwrap(), LinearRgba::new(3.0, 6.0, 0.0, 1.0)),
            (Srgba::hex("0088ff").unwrap(), LinearRgba::new(0.0, 3.0, 6.0, 1.0)),
            (Srgba::hex("ff6600").unwrap(), LinearRgba::new(6.0, 2.0, 0.0, 1.0)),
            (Srgba::hex("aa00ff").unwrap(), LinearRgba::new(4.0, 0.0, 6.0, 1.0)),
        ];

        for i in current..desired {
            let car_index = car_options[i % car_options.len()];
            let blaster_index = blaster_options[i % blaster_options.len()];
            let (bullet_color, bullet_emissive) = bullet_colors[i % bullet_colors.len()];

            let angle = i as f32 * std::f32::consts::TAU / desired as f32;
            let radius = 40.0;
            let pos = Vec3::new(angle.cos() * radius, 3.0, angle.sin() * radius);
            let def = &CAR_DEFS[car_index];
            let car_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(def.path));
            let blaster_def = &BLASTER_DEFS[blaster_index];
            let blaster_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(blaster_def.path));
            let half_height = def.collider.y * 0.5;
            let mount = Vec3::new(0.0, mount_y(def.collider.y), 0.0);

            let ai_root = commands.spawn((
                AiCar,
                AiSpawnMarker,
                RigidBody::Dynamic,
                Position(pos),
                Rotation(Quat::from_rotation_y(f32::atan2(-pos.x, -pos.z))),
                LinearVelocity::ZERO,
                AngularVelocity::ZERO,
                LinearDamping(0.5),
                AngularDamping(1.0),
                MaxLinearSpeed(80.0),
                MaxAngularSpeed(4.0),
                CenterOfMass(Vec3::ZERO),
                Friction::new(0.01),
                SweptCcd::NON_LINEAR,
                Mass(6.0),
            )).id();

            let mut rng = rand::rng();
            let shoot_interval = rng.random_range(1.0..4.0);
            commands.entity(ai_root).insert((
                GravityScale(1.0),
                AiShootTimer { timer: Timer::from_seconds(shoot_interval, TimerMode::Repeating) },
                AiConfig { car_index, bullet_color, bullet_emissive },
            ));

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
                    Transform::from_translation(mount)
                        .with_scale(Vec3::splat(blaster_def.scale))
                        .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                    AiBlasterVisual,
                    AiComputePivot,
                    AiPivotCache::default(),
                ));
            });
        }
    }
}

fn ai_compute_pivot(
    mut commands: Commands,
    ai_query: Query<(Entity, &GlobalTransform, &AiConfig), With<AiCar>>,
    blaster_query: Query<(Entity, &GlobalTransform, &Transform), (With<AiBlasterVisual>, With<AiComputePivot>)>,
    parent_query: Query<&ChildOf>,
    children_query: Query<&Children>,
    mesh_query: Query<&GlobalTransform, With<Mesh3d>>,
    mut pivot_cache_query: Query<&mut AiPivotCache, With<AiBlasterVisual>>,
) {
    for (blaster_entity, _blaster_global, blaster_transform) in blaster_query.iter() {
        let mut center_world = Vec3::ZERO;
        let mut count = 0;

        for desc in children_query.iter_descendants(blaster_entity) {
            if let Ok(desc_global) = mesh_query.get(desc) {
                center_world += desc_global.translation();
                count += 1;
            }
        }

        if count == 0 { continue; }
        center_world /= count as f32;

        let Ok(parent) = parent_query.get(blaster_entity) else { continue };
        let parent_entity: Entity = parent.0;
        let Ok((_, ai_global, ai_config)) = ai_query.get(parent_entity) else { continue };

        let center_local = ai_global.affine().inverse().transform_point(center_world);

        let def = &CAR_DEFS[ai_config.car_index];
        let mount = Vec3::new(0.0, mount_y(def.collider.y), 0.0);
        let rot = blaster_transform.rotation;
        let s = blaster_transform.scale.x;

        let pivot = rot.inverse() * (center_local - mount) / s;

        if let Ok(mut pivot_cache) = pivot_cache_query.get_mut(blaster_entity) {
            pivot_cache.pivot = Some(pivot);
        }
        commands.entity(blaster_entity).remove::<AiComputePivot>();
    }
}

fn ai_aim_blaster(
    ai_query: Query<(Entity, &GlobalTransform, &AiConfig), With<AiCar>>,
    player_entity_res: Query<Entity, With<PlayerCar>>,
    all_cars: Query<(Entity, &GlobalTransform), Or<(With<AiCar>, With<PlayerCar>)>>,
    children_query: Query<&Children>,
    mut blaster_query: Query<(&AiPivotCache, &mut Transform), (With<AiBlasterVisual>, Without<AiCar>)>,
) {
    let player_entity = player_entity_res.single().ok();

    for (ai_entity, ai_global, ai_config) in ai_query.iter() {
        let mut best_dist = f32::MAX;
        let mut target_pos = ai_global.translation();
        for (other_entity, other_global) in all_cars.iter() {
            if other_entity == ai_entity { continue; }
            let mut d = (other_global.translation() - ai_global.translation()).length();
            if player_entity == Some(other_entity) { d *= 0.85; }
            if d < best_dist {
                best_dist = d;
                target_pos = other_global.translation();
            }
        }

        let Ok(children) = children_query.get(ai_entity) else { continue };
        for child in children {
            if let Ok((pivot_cache, mut blaster_transform)) = blaster_query.get_mut(*child) {
                let def = &CAR_DEFS[ai_config.car_index];
                let mount = Vec3::new(0.0, mount_y(def.collider.y), 0.0);
                let s = blaster_transform.scale.x;
                let pivot = pivot_cache.pivot.unwrap_or(Vec3::ZERO);

                let ai_pos = ai_global.translation();
                let ai_rot = ai_global.rotation();
                let aim_point = target_pos + Vec3::new(0.0, 1.0, 0.0);

                let blaster_world_mount = ai_pos + ai_rot * mount;
                let local_aim = ai_rot.inverse() * (aim_point - blaster_world_mount);
                if local_aim.length_squared() < 0.01 { continue; }
                let local_dir = local_aim.normalize();
                let yaw = f32::atan2(-local_dir.x, -local_dir.z);
                let horiz_len = Vec2::new(local_dir.x, local_dir.z).length();
                let pitch = f32::atan2(local_dir.y, horiz_len);

                let rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
                blaster_transform.translation = mount - rotation * (s * pivot);
                blaster_transform.rotation = rotation;

                break;
            }
        }
    }
}

fn ai_drive(
    time: Res<Time>,
    mut ai_query: Query<(Entity, &GlobalTransform, &mut LinearVelocity, &mut AngularVelocity), With<AiCar>>,
    player_query: Query<&GlobalTransform, With<PlayerCar>>,
    all_cars: Query<(Entity, &GlobalTransform), Or<(With<AiCar>, With<PlayerCar>)>>,
) {
    let dt = time.delta_secs();
    let car_data: Vec<(Entity, Vec3)> = all_cars.iter()
        .map(|(e, t)| (e, t.translation()))
        .collect();
    let Ok(player_transform) = player_query.single() else { return };
    let player_pos = player_transform.translation();

    for (ai_entity, ai_transform, mut lin_vel, mut ang_vel) in ai_query.iter_mut() {
        let ai_pos = ai_transform.translation();
        let ai_rot = ai_transform.rotation();

        if ai_pos.y < -3.0 {
            lin_vel.0 = Vec3::new(0.0, 5.0, 0.0);
            ang_vel.0 = Vec3::ZERO;
            continue;
        }

        let to_player = player_pos - ai_pos;
        let flat_to_player = Vec3::new(to_player.x, 0.0, to_player.z);
        let player_dist = flat_to_player.length();

        let boredom = if player_dist > 20.0 {
            1.0 + (player_dist - 20.0) / 30.0
        } else {
            player_dist / 20.0
        };

        let chase_dir = if flat_to_player.length_squared() > 1.0 {
            flat_to_player.normalize_or(Vec3::Z) * boredom
        } else {
            Vec3::ZERO
        };

        let mut flee_dir = Vec3::ZERO;
        for (other_entity, other_pos) in &car_data {
            if *other_entity == ai_entity { continue; }
            let away = ai_pos - *other_pos;
            let flat_away = Vec3::new(away.x, 0.0, away.z);
            let d = flat_away.length();
            if d < 15.0 && d > 0.01 {
                flee_dir += flat_away.normalize_or(Vec3::ZERO) * (1.0 - d / 15.0);
            }
        }

        let desired_dir = (chase_dir + flee_dir).normalize_or(flat_to_player.normalize_or(Vec3::Z));

        let forward = ai_rot * Vec3::Z;
        let flat_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or(Vec3::Z);

        let angle_to_target = f32::atan2(
            flat_forward.cross(desired_dir).y,
            flat_forward.dot(desired_dir),
        );

        ang_vel.0 = Vec3::Y * angle_to_target * 4.0;

        let speed_factor = (1.0 - angle_to_target.abs() / std::f32::consts::PI * 0.8).max(0.2);
        let target_speed = 45.0 * speed_factor;

        let current_vel = lin_vel.0;
        let current_flat = Vec3::new(current_vel.x, 0.0, current_vel.z);
        let current_speed = current_flat.dot(flat_forward);

        let accel = (target_speed - current_speed) * (5.0 * dt).min(1.0);
        let new_speed = current_speed + accel;

        lin_vel.0 = flat_forward * new_speed + Vec3::new(0.0, current_vel.y, 0.0);
    }
}

fn ai_shoot(
    time: Res<Time>,
    mut ai_query: Query<(Entity, &GlobalTransform, &AiConfig, &mut AiShootTimer), With<AiCar>>,
    player_entity_res: Query<Entity, With<PlayerCar>>,
    all_cars: Query<(Entity, &GlobalTransform), Or<(With<AiCar>, With<PlayerCar>)>>,
    blaster_global_query: Query<&GlobalTransform, With<AiBlasterVisual>>,
    children_query: Query<&Children>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let car_data: Vec<(Entity, Vec3)> = all_cars.iter()
        .map(|(e, t)| (e, t.translation()))
        .collect();
    let player_entity = player_entity_res.single().ok();

    for (ai_entity, ai_global, ai_config, mut shoot_timer) in ai_query.iter_mut() {
        shoot_timer.timer.tick(time.delta());
        if !shoot_timer.timer.just_finished() { continue; }

        let mut rng = rand::rng();
        let new_interval = rng.random_range(1.0..4.0);
        shoot_timer.timer.set_duration(Duration::from_secs_f32(new_interval));
        shoot_timer.timer.reset();

        let mut best_dist = f32::MAX;
        let mut target_pos = ai_global.translation();
        for (other_entity, other_pos) in &car_data {
            if *other_entity == ai_entity { continue; }
            let mut d = (*other_pos - ai_global.translation()).length();
            if player_entity == Some(*other_entity) { d *= 0.85; }
            if d < best_dist {
                best_dist = d;
                target_pos = *other_pos;
            }
        }

        let aim_point = target_pos + Vec3::new(rng.random_range(-3.0..3.0), rng.random_range(-1.0..2.0), rng.random_range(-3.0..3.0));

        let Ok(children) = children_query.get(ai_entity) else { continue };
        let mut blaster_pos = ai_global.translation();
        for child in children.iter() {
            if let Ok(global) = blaster_global_query.get(child) {
                blaster_pos = global.translation();
                break;
            }
        }

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
                base_color: ai_config.bullet_color.into(),
                emissive: ai_config.bullet_emissive,
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
}