use std::collections::HashSet;
use bevy::prelude::*;
use avian3d::prelude::{Collider, SpatialQuery, ShapeCastConfig, SpatialQueryFilter};
use rand::Rng;
use crate::car::{PlayerCar, AiCar, CarCamera, CarSelection, CAR_DEFS, mount_y, Health};
use crate::{GameState, Team, RoundCountdown};
use crate::NetMode;

#[derive(Clone)]
pub enum BlasterType {
    Single,
    Shotgun { pellets: u32, spread: f32 },
    Burst { count: u32 },
    Sniper,
}

pub struct BlasterDef {
    pub name: &'static str,
    pub path: &'static str,
    pub scale: f32,
    pub blaster_type: BlasterType,
    pub capacity: f32,
    pub reload_speed: f32,
    pub damage: u8,
}

pub const BLASTER_DEFS: &[BlasterDef] = &[
    BlasterDef { name: "Pistol",    path: "models/small pistol.glb",                scale: 3.5, blaster_type: BlasterType::Single,                 capacity: 1.0, reload_speed: 2.0, damage: 1 },
    BlasterDef { name: "SMG",       path: "models/some smg.glb",                    scale: 3.5, blaster_type: BlasterType::Single,                 capacity: 3.0, reload_speed: 2.0, damage: 1 },
    BlasterDef { name: "Shotgun",   path: "models/dual barrel shotgun.glb",         scale: 3.5, blaster_type: BlasterType::Shotgun { pellets: 3, spread: 0.12 }, capacity: 3.0, reload_speed: 2.0, damage: 1 },
    BlasterDef { name: "Sniper",    path: "models/really big sniper rifle.glb",     scale: 3.5, blaster_type: BlasterType::Sniper,                 capacity: 1.0, reload_speed: 0.67, damage: 3 },
    BlasterDef { name: "Quad",      path: "models/quadruple barel pistol, look sreally cool.glb", scale: 3.5, blaster_type: BlasterType::Shotgun { pellets: 3, spread: 0.2 }, capacity: 3.0, reload_speed: 2.0, damage: 1 },
    BlasterDef { name: "Rifle",     path: "models/maybe ar.glb",                    scale: 3.5, blaster_type: BlasterType::Burst { count: 3 }, capacity: 3.0, reload_speed: 2.0, damage: 1 },
];

#[derive(Resource)]
pub struct BlasterSelection {
    pub index: usize,
    pub pending_change: bool,
    pub random: bool,
}

impl Default for BlasterSelection {
    fn default() -> Self {
        Self { index: 0, pending_change: false, random: false }
    }
}

impl BlasterSelection {
    pub fn display_index(&self) -> usize {
        if self.random { 0 } else { self.index }
    }
}

#[derive(Component)]
pub struct BlasterVisual;

#[derive(Component)]
pub struct ComputePivot;

#[derive(Component)]
pub struct Bullet {
    pub velocity: Vec3,
    pub lifetime: Timer,
    pub damage: u8,
}

#[derive(Component)]
pub struct ExcludeMeshRayCast(pub HashSet<Entity>);

#[derive(Resource, Default)]
struct PivotCache {
    pivot: Option<Vec3>,
}

#[derive(Resource, Default)]
struct AimInfo {
    aim_point: Option<Vec3>,
}

pub const BULLET_SPEED: f32 = 150.0;
pub const BULLET_RADIUS: f32 = 0.5;
pub const BULLET_LIFETIME_SECS: f32 = 5.0;

#[derive(Resource, Default)]
pub struct WeaponCharge(pub f32);

fn not_client(mode: Res<NetMode>) -> bool {
    !matches!(*mode, NetMode::Client)
}

pub struct BlasterPlugin;

impl Plugin for BlasterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BlasterSelection>()
            .init_resource::<PivotCache>()
            .init_resource::<AimInfo>()
            .init_resource::<WeaponCharge>()
            .add_systems(Update, (
                switch_blaster,
                compute_pivot,
                aim_blaster,
            ).chain().run_if(in_state(GameState::Playing).and(not_client)))
            .add_systems(Update, (player_shoot, move_bullets).chain().run_if(in_state(GameState::Playing).and(not_client)));
    }
}

fn compute_pivot(
    mut commands: Commands,
    mut pivot_cache: ResMut<PivotCache>,
    car_query: Query<&GlobalTransform, With<PlayerCar>>,
    blaster_query: Query<(Entity, &GlobalTransform, &Transform), (With<BlasterVisual>, With<ComputePivot>)>,
    children_query: Query<&Children>,
    mesh_query: Query<&GlobalTransform, With<Mesh3d>>,
    car_selection: Res<CarSelection>,
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

        let Ok(car_global) = car_query.single() else { continue };
        let center_car_local = car_global.affine().inverse().transform_point(center_world);

        let car_def = &CAR_DEFS[car_selection.display_index()];
        let mount = Vec3::new(0.0, mount_y(car_def.collider.y), 0.0);
        let rot = blaster_transform.rotation;
        let s = blaster_transform.scale.x;

        let pivot = rot.inverse() * (center_car_local - mount) / s;

        pivot_cache.pivot = Some(pivot);
        commands.entity(entity).remove::<ComputePivot>();
        return;
    }
}

fn aim_blaster(
    car_query: Query<(Entity, &GlobalTransform), With<PlayerCar>>,
    car_selection: Res<CarSelection>,
    pivot_cache: Res<PivotCache>,
    mut blaster_query: Query<&mut Transform, (With<BlasterVisual>, Without<PlayerCar>)>,
    camera_query: Query<(&Camera, &GlobalTransform), With<CarCamera>>,
    windows: Query<&Window>,
    children_query: Query<&Children>,
    mut mesh_ray_cast: MeshRayCast,
    mut aim_info: ResMut<AimInfo>,
) {
    let Ok((car_entity, car_global)) = car_query.single() else { return };
    let Ok(mut blaster) = blaster_query.single_mut() else { return };
    let Ok((camera, cam_global)) = camera_query.single() else { return };
    let Ok(window) = windows.single() else { return };

    let car_def = &CAR_DEFS[car_selection.display_index()];
    let mount = Vec3::new(0.0, mount_y(car_def.collider.y), 0.0);
    let s = blaster.scale.x;
    let pivot = pivot_cache.pivot.unwrap_or(Vec3::ZERO);

    let Some(cursor) = window.cursor_position() else {
        let rotation = blaster.rotation;
        blaster.translation = mount - rotation * (s * pivot);
        aim_info.aim_point = None;
        return;
    };
    let Ok(ray) = camera.viewport_to_world(cam_global, cursor) else {
        aim_info.aim_point = None;
        return;
    };

    let mut exclude = HashSet::new();
    exclude.insert(car_entity);
    for desc in children_query.iter_descendants(car_entity) {
        exclude.insert(desc);
    }

    let ray3d = Ray3d::new(ray.origin, ray.direction);
    let filter = |entity: Entity| !exclude.contains(&entity);
    let settings = MeshRayCastSettings::default()
        .with_visibility(RayCastVisibility::Any)
        .with_filter(&filter)
        .with_early_exit_test(&|_| true);
    let aim = if let Some((_, hit)) = mesh_ray_cast.cast_ray(ray3d, &settings).first() {
        hit.point
    } else {
        let t = -ray.origin.y / ray.direction.y;
        if t <= 0.0 {
            aim_info.aim_point = None;
            return;
        }
        ray.origin + ray.direction * t
    };

    aim_info.aim_point = Some(aim);

    let car_pos = car_global.translation();
    let car_rot = car_global.rotation();
    let blaster_world_mount = car_pos + car_rot * mount;
    let local_aim = car_rot.inverse() * (aim - blaster_world_mount);
    if local_aim.length_squared() < 0.01 { return; }
    let local_dir = local_aim.normalize();
    let yaw = f32::atan2(-local_dir.x, -local_dir.z);
    let horiz_len = Vec2::new(local_dir.x, local_dir.z).length();
    let pitch = f32::atan2(local_dir.y, horiz_len);

    let rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    blaster.translation = mount - rotation * (s * pivot);
    blaster.rotation = rotation;
}

pub fn spawn_bullet(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    direction: Vec3,
    damage: u8,
    exclude: HashSet<Entity>,
    color: Srgba,
    emissive: LinearRgba,
    bullet_owner: Option<crate::BulletOwner>,
) {
    let mut cmd = commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.8).mesh().ico(2).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color.into(),
            emissive,
            ..default()
        })),
        Transform::from_translation(position),
        Bullet {
            velocity: direction * BULLET_SPEED,
            lifetime: Timer::from_seconds(BULLET_LIFETIME_SECS, TimerMode::Once),
            damage,
        },
        ExcludeMeshRayCast(exclude),
    ));
    if let Some(bo) = bullet_owner {
        cmd.insert(bo);
    }
}

fn player_shoot(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    aim_info: Res<AimInfo>,
    blaster_query: Query<&GlobalTransform, With<BlasterVisual>>,
    car_query: Query<Entity, With<PlayerCar>>,
    children_query: Query<&Children>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    blaster_selection: Res<BlasterSelection>,
    mut charge: ResMut<WeaponCharge>,
    countdown: Option<Res<RoundCountdown>>,
) {
    if let Some(cd) = countdown { if cd.0.remaining_secs() > 0.0 { return; } }
    let def = &BLASTER_DEFS[blaster_selection.display_index()];

    charge.0 = (charge.0 + def.reload_speed * time.delta_secs()).min(def.capacity);

    if !mouse_buttons.pressed(MouseButton::Left) { return; }

    let Ok(blaster_global) = blaster_query.single() else { return };
    let Some(aim_point) = aim_info.aim_point else { return };

    let blaster_pos = blaster_global.translation();
    let base_dir = (aim_point - blaster_pos).normalize_or(*blaster_global.forward());
    let spawn_pos = blaster_pos + base_dir * 1.0;

    let mut exclude = HashSet::new();
    let Ok(car_entity) = car_query.single() else { return };
    exclude.insert(car_entity);
    for desc in children_query.iter_descendants(car_entity) {
        exclude.insert(desc);
    }

    let color = Srgba::hex("ff0000").unwrap();
    let emissive = LinearRgba::new(8.0, 0.0, 0.0, 1.0);
    let bullet_owner = Some(crate::BulletOwner { client_id: 0, team: 0 });
    let shot_cost = match &def.blaster_type {
        BlasterType::Single | BlasterType::Sniper => 1.0,
        BlasterType::Shotgun { pellets, .. } => *pellets as f32,
        BlasterType::Burst { count, .. } => *count as f32,
    };

    if charge.0 < shot_cost { return; }
    charge.0 -= shot_cost;

    match &def.blaster_type {
        BlasterType::Single | BlasterType::Sniper => {
            spawn_bullet(&mut commands, &mut meshes, &mut materials, spawn_pos, base_dir, def.damage, exclude, color, emissive, bullet_owner);
        }
        BlasterType::Shotgun { pellets, spread } => {
            let pellets = *pellets;
            let spread = *spread;
            let mut rng = rand::rng();
            for _ in 0..pellets {
                let s = Vec3::new(rng.random_range(-spread..spread), rng.random_range(-spread..spread), rng.random_range(-spread..spread));
                let dir = (base_dir + s).normalize_or(base_dir);
                spawn_bullet(&mut commands, &mut meshes, &mut materials, spawn_pos, dir, def.damage, exclude.clone(), color, emissive, bullet_owner.clone());
            }
        }
        BlasterType::Burst { count, .. } => {
            let count = *count;
            for _ in 0..count {
                spawn_bullet(&mut commands, &mut meshes, &mut materials, spawn_pos, base_dir, def.damage, exclude.clone(), color, emissive, bullet_owner.clone());
            }
        }
    }
}

fn move_bullets(
    mut bullet_query: Query<(Entity, &mut Transform, &mut Bullet, &ExcludeMeshRayCast, Option<&crate::BulletOwner>)>,
    time: Res<Time>,
    spatial_query: SpatialQuery,
    parent_query: Query<&ChildOf>,
    player_query: Query<(), With<PlayerCar>>,
    ai_query: Query<(), With<AiCar>>,
    mut health_query: Query<&mut Health>,
    team_query: Query<&Team>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut transform, mut bullet, exclude_ray, bullet_owner) in bullet_query.iter_mut() {
        
    // Original code continues...

        bullet.lifetime.tick(time.delta());
        if bullet.lifetime.just_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let prev_pos = transform.translation;
        let delta = bullet.velocity * time.delta_secs();
        let distance = delta.length();

        if distance < 0.001 {
            commands.entity(entity).despawn();
            continue;
        }

        let Ok(direction) = Dir3::new(delta) else {
            commands.entity(entity).despawn();
            continue;
        };

        let shape = Collider::sphere(BULLET_RADIUS);
        let config = ShapeCastConfig::from_max_distance(distance);
        let filter = SpatialQueryFilter::from_excluded_entities(exclude_ray.0.iter().copied());

        if let Some(hit) = spatial_query.cast_shape(&shape, prev_pos, Quat::IDENTITY, direction, &config, &filter) {
            if let Some(car_entity) = find_car_ancestor(hit.entity, &parent_query, &player_query, &ai_query) {
                if let Ok(mut health) = health_query.get_mut(car_entity) {
                    let friendly = bullet_owner.and_then(|bo| team_query.get(car_entity).ok().map(|t| bo.team == t.0 && bo.team != 0)).unwrap_or(false);
                    if !friendly {
                        health.0 = health.0.saturating_sub(bullet.damage);
                    }
                }
                transform.translation = prev_pos + direction * hit.distance;
                commands.entity(entity).despawn();
                continue;
            }
            let hit_pos = prev_pos + direction * hit.distance;
            spawn_smoke_effect(&mut commands, &mut meshes, &mut materials, hit_pos, 3);
            transform.translation = hit_pos;
            commands.entity(entity).despawn();
            continue;
        }

        transform.translation = prev_pos + delta;
    }
}

fn find_car_ancestor(
    start: Entity,
    parent_query: &Query<&ChildOf>,
    player_query: &Query<(), With<PlayerCar>>,
    ai_query: &Query<(), With<AiCar>>,
) -> Option<Entity> {
    let mut current = start;
    for _ in 0..32 {
        if player_query.get(current).is_ok() || ai_query.get(current).is_ok() {
            return Some(current);
        }
        match parent_query.get(current) {
            Ok(child_of) => current = child_of.0,
            Err(_) => return None,
        }
    }
    None
}

pub fn spawn_smoke_effect(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    count: u32,
) {
    let mut rng = rand::rng();
    for _ in 0..count.min(3) {
        let dir = Vec3::new(
            rng.random_range(-0.5..0.5),
            rng.random_range(0.0..0.5),
            rng.random_range(-0.5..0.5),
        ).normalize_or(Vec3::Y);
        let speed = rng.random_range(1.0..4.0);
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.3).mesh().ico(1).unwrap())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Srgba::hex("ff6600").unwrap().into(),
                emissive: LinearRgba::new(2.0, 1.0, 0.0, 1.0),
                ..default()
            })),
            Transform::from_translation(position),
            crate::car::ExplosionParticle {
                velocity: dir * speed,
                lifetime: Timer::from_seconds(rng.random_range(0.5..1.0), TimerMode::Once),
            },
        ));
    }
}

fn switch_blaster(
    mut selection: ResMut<BlasterSelection>,
    car_selection: Res<CarSelection>,
    car_query: Query<Entity, With<PlayerCar>>,
    blaster_query: Query<Entity, With<BlasterVisual>>,
    children_query: Query<&Children>,
    mut pivot_cache: ResMut<PivotCache>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if !selection.pending_change {
        return;
    }
    selection.pending_change = false;

    let Ok(car_entity) = car_query.single() else {
        return;
    };

    for child in children_query.iter_descendants(car_entity) {
        if blaster_query.get(child).is_ok() {
            commands.entity(child).despawn();
        }
    }

    let blaster_def = &BLASTER_DEFS[selection.display_index()];
    let car_def = &CAR_DEFS[car_selection.display_index()];
    let mount = Vec3::new(0.0, mount_y(car_def.collider.y), 0.0);
    let scale = blaster_def.scale;

    let scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(blaster_def.path));
    commands.entity(car_entity).with_children(|parent| {
        parent.spawn((
            SceneRoot(scene),
            Transform::from_translation(mount).with_scale(Vec3::splat(scale)),
            BlasterVisual,
            ComputePivot,
        ));
    });

    pivot_cache.pivot = None;
}
