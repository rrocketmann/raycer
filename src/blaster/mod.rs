use bevy::prelude::*;
use avian3d::prelude::Rotation;
use crate::car::{PlayerCar, CarCamera, CarSelection, CAR_DEFS, mount_y};

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

#[derive(Resource)]
pub struct BlasterSelection {
    pub index: usize,
    pub pending_change: bool,
}

impl Default for BlasterSelection {
    fn default() -> Self {
        Self { index: 0, pending_change: true }
    }
}

#[derive(Component)]
pub struct BlasterVisual;

#[derive(Component)]
struct ComputePivot;

pub struct BlasterPlugin;

#[derive(Resource, Default)]
struct PivotCache {
    pivot: Option<Vec3>,
}

impl Plugin for BlasterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BlasterSelection>()
            .init_resource::<PivotCache>()
            .add_systems(Update, (
                switch_blaster,
                compute_pivot,
                aim_blaster,
            ).chain());
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
    car_query: Query<&Rotation, With<PlayerCar>>,
    car_selection: Res<CarSelection>,
    pivot_cache: Res<PivotCache>,
    mut blaster_query: Query<&mut Transform, (With<BlasterVisual>, Without<PlayerCar>)>,
    camera_query: Query<(&Camera, &GlobalTransform), With<CarCamera>>,
    windows: Query<&Window>,
) {
    let Ok(car_rot) = car_query.single() else { return };
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
        return;
    };
    let Ok(ray) = camera.viewport_to_world(cam_global, cursor) else {
        return;
    };

    let t = -ray.origin.y / ray.direction.y;
    if t <= 0.0 { return; }
    let aim = ray.origin + ray.direction * t;

    let car_mat = car_rot.0;
    let blaster_world_mount = car_mat * mount;
    let local_aim = car_mat.inverse() * (aim - blaster_world_mount);
    if local_aim.length_squared() < 0.01 { return; }
    let local_dir = local_aim.normalize();
    let yaw = f32::atan2(-local_dir.x, -local_dir.z);
    let horiz_len = Vec2::new(local_dir.x, local_dir.z).length();
    let pitch = f32::atan2(local_dir.y, horiz_len);

    let rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    blaster.translation = mount - rotation * (s * pivot);
    blaster.rotation = rotation;
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