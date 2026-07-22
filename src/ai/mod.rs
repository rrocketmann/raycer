use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::HashSet;
use crate::blaster::{BLASTER_DEFS, spawn_smoke_effect};
use crate::car::{AiCar, CarVisual, PlayerCar, CAR_DEFS, mount_y, Health, spawn_health_indicators, ExplosionTimer, DamageTracker};
use crate::GameState;
use crate::AiEnemyCount;
use crate::MaxHealthPoints;
use crate::NetMode;
use crate::{CarModelIndex, BlasterModelIndex, Team};
use rand::Rng;

#[derive(Component)]
pub struct AiBlasterVisual;

#[derive(Component)]
struct AiConfig {
    car_index: usize,
    blaster_index: usize,
}

#[derive(Component)]
struct AiWeaponCharge(pub f32);

fn not_client(mode: Res<NetMode>) -> bool {
    !matches!(*mode, NetMode::Client)
}

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnExit(GameState::PreGame), cleanup_ai_cars)
            .add_systems(OnEnter(GameState::Playing), spawn_ai_cars)
            .add_systems(OnExit(GameState::Eliminated), cleanup_ai_cars)
            .add_systems(Update, ai_aim_blaster)
            .add_systems(Update, ai_drive.run_if(in_state(GameState::Playing).and(not_client)))
            .add_systems(Update, ai_shoot.run_if(in_state(GameState::Playing).and(not_client)))
            .add_systems(Update, despawn_dead_cars.run_if(in_state(GameState::Playing).and(not_client)))
            .add_systems(Update, damage_smoke.run_if(in_state(GameState::Playing).and(not_client)));
    }
}

fn spawn_ai_cars(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    enemy_count: Res<AiEnemyCount>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    max_hp: Res<MaxHealthPoints>,
) {
    let car_options: Vec<usize> = vec![3, 5, 7, 8, 9, 10, 0, 1];
    let blaster_options: Vec<usize> = vec![1, 2, 3, 4, 0];
    let count = enemy_count.count;
    for i in 0..count {
        let car_index = car_options[i % car_options.len()];
        let blaster_index = blaster_options[i % blaster_options.len()];


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
        )).insert((
            Health(max_hp.hp),
            DamageTracker::default(),
        )).id();
        spawn_health_indicators(ai_root, &mut commands, &mut meshes, &mut materials, def.collider.y, max_hp.hp);

        commands.entity(ai_root).insert((
            GravityScale(1.0),
            AiWeaponCharge(0.0),
            AiConfig { car_index, blaster_index },
            CarModelIndex(car_index),
            BlasterModelIndex(blaster_index),
            Team(0),
        ));

        commands.entity(ai_root).with_children(|parent| {
            parent.spawn((
                Collider::round_cuboid(def.collider.x - 0.2, def.collider.y - 0.2, def.collider.z - 0.2, 0.1),
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
            ));
        });
    }
}

fn cleanup_ai_cars(mut commands: Commands, q: Query<Entity, With<AiSpawnMarker>>) {
    for e in q.iter() {
        commands.entity(e).despawn();
    }
}

#[derive(Component)]
struct AiSpawnMarker;

fn ai_aim_blaster(
    ai_query: Query<(Entity, &GlobalTransform, &AiConfig), With<AiCar>>,
    player_query: Query<&GlobalTransform, With<PlayerCar>>,
    children_query: Query<&Children>,
    mut blaster_query: Query<&mut Transform, (With<AiBlasterVisual>, Without<AiCar>)>,
) {
    let Ok(player_global) = player_query.single() else { return };
    let target_pos = player_global.translation();

    for (ai_entity, ai_global, ai_config) in ai_query.iter() {
        let Ok(children) = children_query.get(ai_entity) else { continue };
        for child in children {
            if let Ok(mut blaster_transform) = blaster_query.get_mut(*child) {
                let def = &CAR_DEFS[ai_config.car_index];
                let mount_y_val = mount_y(def.collider.y);
                let mount = Vec3::new(0.0, mount_y_val, 0.0);
                let s = blaster_transform.scale.x;
                let centroid_y = def.collider.y + 0.2;
                let pivot = Vec3::new(0.0, (centroid_y - mount_y_val) / s, 0.0);

                let ai_pos = ai_global.translation();
                let ai_rot = ai_global.rotation();
                let aim_point = target_pos + Vec3::new(0.0, 1.0, 0.0);
                let local_aim = ai_rot.inverse() * (aim_point - ai_pos);
                if local_aim.length_squared() < 0.01 { continue; }
                let local_dir = local_aim.normalize();
                let yaw = f32::atan2(-local_dir.x, -local_dir.z);
                let horiz_len = Vec2::new(local_dir.x, local_dir.z).length();
                let pitch = f32::atan2(local_dir.y, horiz_len);
                let rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
                blaster_transform.translation = mount - rotation * (s * pivot);
                blaster_transform.rotation = rotation;
            }
        }
    }
}

fn ai_shoot(
    time: Res<Time>,
    mut ai_query: Query<(Entity, &GlobalTransform, &AiConfig, &mut AiWeaponCharge), With<AiCar>>,
    player_query: Query<&GlobalTransform, With<PlayerCar>>,
    blaster_global_query: Query<&GlobalTransform, With<AiBlasterVisual>>,
    children_query: Query<&Children>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(player_global) = player_query.single() else { return };
    let target_pos = player_global.translation();
    let color = Srgba::hex("ff0000").unwrap();
    let emissive = LinearRgba::new(8.0, 0.0, 0.0, 1.0);

    for (ai_entity, ai_global, ai_config, mut charge) in ai_query.iter_mut() {
        let blaster_def = &BLASTER_DEFS[ai_config.blaster_index];
        charge.0 = (charge.0 + blaster_def.reload_speed * time.delta_secs()).min(blaster_def.capacity);
        let shot_cost = match &blaster_def.blaster_type {
            crate::blaster::BlasterType::Single | crate::blaster::BlasterType::Sniper => 1.0,
            crate::blaster::BlasterType::Shotgun { pellets, .. } => *pellets as f32,
            crate::blaster::BlasterType::Burst { count, .. } => *count as f32,
        };
        if charge.0 < shot_cost { continue; }
        charge.0 -= shot_cost;

        let mut rng = rand::rng();
        let aim_point = target_pos + Vec3::new(rng.random_range(-0.4..0.4), rng.random_range(0.8..1.5), rng.random_range(-0.4..0.4));

        let Ok(children) = children_query.get(ai_entity) else { continue };
        let mut blaster_pos = ai_global.translation();
        for child in children.iter() {
            if let Ok(global) = blaster_global_query.get(child) {
                blaster_pos = global.translation();
                break;
            }
        }

        let base_dir = (aim_point - blaster_pos).normalize_or(Vec3::Z);
        let spawn_pos = blaster_pos + base_dir * 1.0;

        let mut exclude = HashSet::new();
        exclude.insert(ai_entity);
        for desc in children_query.iter_descendants(ai_entity) {
            exclude.insert(desc);
        }

        let bo: Option<crate::BulletOwner> = None;
        match &blaster_def.blaster_type {
            crate::blaster::BlasterType::Single | crate::blaster::BlasterType::Sniper => {
                crate::blaster::spawn_bullet(&mut commands, &mut meshes, &mut materials, spawn_pos, base_dir, blaster_def.damage, exclude, color, emissive, bo);
            }
            crate::blaster::BlasterType::Shotgun { pellets, spread } => {
                let pellets = *pellets;
                let spread = *spread;
                for _ in 0..pellets {
                    let s = Vec3::new(rng.random_range(-spread..spread), rng.random_range(-spread..spread), rng.random_range(-spread..spread));
                    let dir = (base_dir + s).normalize_or(base_dir);
                    crate::blaster::spawn_bullet(&mut commands, &mut meshes, &mut materials, spawn_pos, dir, blaster_def.damage, exclude.clone(), color, emissive, bo.clone());
                }
            }
            crate::blaster::BlasterType::Burst { count, .. } => {
                let count = *count;
                for _ in 0..count {
                    crate::blaster::spawn_bullet(&mut commands, &mut meshes, &mut materials, spawn_pos, base_dir, blaster_def.damage, exclude.clone(), color, emissive, bo.clone());
                }
            }
        }
    }
}

fn damage_smoke(
    mut commands: Commands,
    mut car_query: Query<(Entity, &Health, &mut DamageTracker, &GlobalTransform), (With<AiCar>, Without<PlayerCar>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    max_hp: Res<MaxHealthPoints>,
) {
    for (_entity, health, mut tracker, transform) in car_query.iter_mut() {
        let damage_taken = max_hp.hp.saturating_sub(health.0);
        if damage_taken > tracker.total_damage_taken {
            let new_damage = damage_taken - tracker.total_damage_taken;
            tracker.total_damage_taken = damage_taken;
            let smoke_count = (new_damage as u32) * 3;
            spawn_smoke_effect(&mut commands, &mut meshes, &mut materials, transform.translation(), smoke_count);
        }
    }
}

fn ai_drive(
    mut ai_query: Query<(&AiConfig, &mut LinearVelocity, &Position, &Rotation), With<AiCar>>,
    player_query: Query<&Position, (With<PlayerCar>, Without<AiCar>)>,
    time: Res<Time>,
) {
    let Ok(player_pos) = player_query.single() else { return };
    let max_speed = 60.0;

    for (_config, mut velocity, pos, rot) in ai_query.iter_mut() {
        let to_player = player_pos.0 - pos.0;
        let dist = to_player.length();

        if dist < 15.0 {
            let flee_dir = -to_player.normalize_or(Vec3::Z);
            let flat_flee = Vec3::new(flee_dir.x, 0.0, flee_dir.z).normalize_or(Vec3::Z);
            let target = flat_flee * max_speed;
            velocity.0 = velocity.0.lerp(target, time.delta_secs() * 3.0);
            continue;
        }

        let flat_to = Vec3::new(to_player.x, 0.0, to_player.z).normalize_or(Vec3::Z);
        let target = flat_to * max_speed;
        velocity.0 = velocity.0.lerp(target, time.delta_secs() * 2.0);

        let _target_yaw = f32::atan2(-flat_to.x, -flat_to.z);
        let _current_yaw = rot.to_euler(EulerRot::YXZ).0;
    }
}

fn despawn_dead_cars(
    mut commands: Commands,
    ai_query: Query<(Entity, &Health, &Position), With<AiCar>>,
    exploding_query: Query<&ExplosionTimer>,
) {
    for (entity, health, pos) in ai_query.iter() {
        if pos.0.y < -20.0 || (health.0 == 0 && exploding_query.get(entity).is_err()) {
            commands.entity(entity).insert((
                ExplosionTimer(Timer::from_seconds(0.4, TimerMode::Once)),
                LinearVelocity::ZERO,
                AngularVelocity::ZERO,
            ));
        }
    }
}
