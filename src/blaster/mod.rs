use std::collections::HashSet;
use bevy::prelude::*;
use avian3d::prelude::{Collider, SpatialQuery, ShapeCastConfig, SpatialQueryFilter};
use crate::car::{PlayerCar, AiCar, CarCamera, CarSelection, CAR_DEFS, mount_y, Health};
use crate::GameState;
use crate::RubberBullets;

pub struct BlasterDef {
    pub name: &'static str,
    pub path: &'static str,
    pub scale: f32,
}

pub const BLASTER_DEFS: &[BlasterDef] = &[
    BlasterDef { name: "Blaster A", path: "models/blaster-a.glb", scale: 2.0 },
    BlasterDef { name: "Blaster B", path: "models/blaster-b.glb", scale: 2.0 },
    BlasterDef { name: "Blaster C", path: "models/blaster-c.glb", scale: 2.0 },
    BlasterDef { name: "Blaster D", path: "models/blaster-d.glb", scale: 2.0 },
    BlasterDef { name: "Blaster E", path: "models/blaster-e.glb", scale: 2.0 },
    BlasterDef { name: "Blaster F", path: "models/blaster-f.glb", scale: 2.0 },
    BlasterDef { name: "Blaster G", path: "models/blaster-g.glb", scale: 2.0 },
    BlasterDef { name: "Blaster H", path: "models/blaster-h.glb", scale: 2.0 },
    BlasterDef { name: "Blaster I", path: "models/blaster-i.glb", scale: 2.0 },
    BlasterDef { name: "Blaster J", path: "models/blaster-j.glb", scale: 2.0 },
    BlasterDef { name: "Blaster K", path: "models/blaster-k.glb", scale: 2.0 },
    BlasterDef { name: "Blaster L", path: "models/blaster-l.glb", scale: 2.0 },
    BlasterDef { name: "Blaster M", path: "models/blaster-m.glb", scale: 2.0 },
    BlasterDef { name: "Blaster N", path: "models/blaster-n.glb", scale: 2.0 },
    BlasterDef { name: "Blaster O", path: "models/blaster-o.glb", scale: 2.0 },
    BlasterDef { name: "Blaster P", path: "models/blaster-p.glb", scale: 2.0 },
    BlasterDef { name: "Blaster Q", path: "models/blaster-q.glb", scale: 2.0 },
    BlasterDef { name: "Blaster R", path: "models/blaster-r.glb", scale: 2.0 },
];

#[derive(Resource, Default)]
pub struct BlasterSelection {
    pub index: usize,
    pub pending_change: bool,
}

#[derive(Component)]
pub struct BlasterVisual;

#[derive(Component)]
pub struct ComputePivot;

#[derive(Component)]
pub struct Bullet {
    pub velocity: Vec3,
    pub lifetime: Timer,
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

pub const BULLET_SPEED: f32 = 80.0;
pub const BULLET_RADIUS: f32 = 0.5;
pub const BULLET_LIFETIME_SECS: f32 = 5.0;

pub struct BlasterPlugin;

impl Plugin for BlasterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BlasterSelection>()
            .init_resource::<PivotCache>()
            .init_resource::<AimInfo>()
            .add_systems(Update, (
                switch_blaster,
                compute_pivot,
                aim_blaster,
            ).chain())
            .add_systems(Update, (shoot_bullet, move_bullets).chain().run_if(in_state(GameState::Playing)));
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

        let car_def = &CAR_DEFS[car_selection.index];
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

    let car_def = &CAR_DEFS[car_selection.index];
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

fn shoot_bullet(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    aim_info: Res<AimInfo>,
    blaster_query: Query<&GlobalTransform, With<BlasterVisual>>,
    car_query: Query<Entity, With<PlayerCar>>,
    children_query: Query<&Children>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(blaster_global) = blaster_query.single() else { return };
    let Some(aim_point) = aim_info.aim_point else { return };

    let blaster_pos = blaster_global.translation();
    let direction = (aim_point - blaster_pos).normalize_or(*blaster_global.forward());
    let spawn_pos = blaster_pos + direction * 1.0;

    let mut exclude = HashSet::new();
    let Ok(car_entity) = car_query.single() else { return };
    exclude.insert(car_entity);
    for desc in children_query.iter_descendants(car_entity) {
        exclude.insert(desc);
    }

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(BULLET_RADIUS).mesh().ico(2).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Srgba::hex("ff0000").unwrap().into(),
            emissive: LinearRgba::new(8.0, 0.0, 0.0, 1.0),
            ..default()
        })),
        Transform::from_translation(spawn_pos),
        Bullet {
            velocity: direction * BULLET_SPEED,
            lifetime: Timer::from_seconds(BULLET_LIFETIME_SECS, TimerMode::Once),
        },
        ExcludeMeshRayCast(exclude),
    ));
}

fn move_bullets(
    mut bullet_query: Query<(Entity, &mut Transform, &mut Bullet, &ExcludeMeshRayCast)>,
    time: Res<Time>,
    spatial_query: SpatialQuery,
    parent_query: Query<&ChildOf>,
    player_query: Query<(), With<PlayerCar>>,
    ai_query: Query<(), With<AiCar>>,
    mut health_query: Query<&mut Health>,
    rubber_bullets: Res<RubberBullets>,
    mut commands: Commands,
) {
    for (entity, mut transform, mut bullet, exclude_ray) in bullet_query.iter_mut() {
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
                    health.0 = health.0.saturating_sub(1);
                }
                transform.translation = prev_pos + direction * hit.distance;
                commands.entity(entity).despawn();
                continue;
            }
            if rubber_bullets.0 {
                let reflected = direction.as_vec3().reflect(hit.normal1).normalize_or_zero();
                if let Ok(dir) = Dir3::new(reflected) {
                    bullet.velocity = dir * BULLET_SPEED;
                }
                let safe_dist = (hit.distance - BULLET_RADIUS).max(0.0);
                transform.translation = prev_pos + direction * safe_dist;
                continue;
            }
            transform.translation = prev_pos + direction * hit.distance;
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

    let blaster_def = &BLASTER_DEFS[selection.index];
    let car_def = &CAR_DEFS[car_selection.index];
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