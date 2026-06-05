use bevy::prelude::*;
use avian3d::prelude::Rotation;
use crate::car::{PlayerCar, CarCamera};

pub struct BlasterDef {
    pub name: &'static str,
    pub path: &'static str,
}

pub const BLASTER_DEFS: &[BlasterDef] = &[
    BlasterDef { name: "Blaster A", path: "models/blaster-a.glb" },
    BlasterDef { name: "Blaster B", path: "models/blaster-b.glb" },
    BlasterDef { name: "Blaster C", path: "models/blaster-c.glb" },
    BlasterDef { name: "Blaster D", path: "models/blaster-d.glb" },
    BlasterDef { name: "Blaster E", path: "models/blaster-e.glb" },
    BlasterDef { name: "Blaster F", path: "models/blaster-f.glb" },
    BlasterDef { name: "Blaster G", path: "models/blaster-g.glb" },
    BlasterDef { name: "Blaster H", path: "models/blaster-h.glb" },
    BlasterDef { name: "Blaster I", path: "models/blaster-i.glb" },
    BlasterDef { name: "Blaster J", path: "models/blaster-j.glb" },
    BlasterDef { name: "Blaster K", path: "models/blaster-k.glb" },
    BlasterDef { name: "Blaster L", path: "models/blaster-l.glb" },
    BlasterDef { name: "Blaster M", path: "models/blaster-m.glb" },
    BlasterDef { name: "Blaster N", path: "models/blaster-n.glb" },
    BlasterDef { name: "Blaster O", path: "models/blaster-o.glb" },
    BlasterDef { name: "Blaster P", path: "models/blaster-p.glb" },
    BlasterDef { name: "Blaster Q", path: "models/blaster-q.glb" },
    BlasterDef { name: "Blaster R", path: "models/blaster-r.glb" },
];

#[derive(Resource, Default)]
pub struct BlasterSelection {
    pub index: usize,
    pub pending_change: bool,
}

#[derive(Component)]
pub struct BlasterVisual;

pub struct BlasterPlugin;

impl Plugin for BlasterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BlasterSelection>()
            .add_systems(Startup, spawn_initial_blaster)
            .add_systems(Update, (
                switch_blaster,
                aim_blaster,
            ));
    }
}

fn spawn_initial_blaster(
    mut commands: Commands,
    car_query: Query<Entity, With<PlayerCar>>,
    asset_server: Res<AssetServer>,
) {
    let Ok(car_entity) = car_query.single() else {
        return;
    };
    let def = &BLASTER_DEFS[0];
    let scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(def.path));
    commands.entity(car_entity).with_children(|parent| {
        parent.spawn((
            SceneRoot(scene),
            Transform::from_translation(Vec3::new(0.0, 1.2, 0.0)).with_scale(Vec3::splat(2.0)),
            BlasterVisual,
        ));
    });
}

fn aim_blaster(
    car_query: Query<&Rotation, With<PlayerCar>>,
    mut blaster_query: Query<(&mut Transform, &GlobalTransform), (With<BlasterVisual>, Without<PlayerCar>)>,
    camera_query: Query<(&Camera, &GlobalTransform), With<CarCamera>>,
    windows: Query<&Window>,
) {
    let Ok(car_rot) = car_query.single() else { return };
    let Ok((mut blaster, blaster_global)) = blaster_query.single_mut() else { return };
    let Ok((camera, cam_global)) = camera_query.single() else { return };
    let Ok(window) = windows.single() else { return };

    let Some(cursor) = window.cursor_position() else { return };
    let Ok(ray) = camera.viewport_to_world(cam_global, cursor) else { return };

    let t = -ray.origin.y / ray.direction.y;
    if t <= 0.0 { return; }
    let aim = ray.origin + ray.direction * t;

    let blaster_world_pos = blaster_global.translation();
    let world_dir = aim - blaster_world_pos;
    if world_dir.length_squared() < 0.01 { return; }
    let world_dir = world_dir.normalize();

    let local_dir = car_rot.0.inverse() * world_dir;
    let yaw = f32::atan2(-local_dir.x, -local_dir.z);
    let horiz_len = Vec2::new(local_dir.x, local_dir.z).length();
    let pitch = f32::atan2(local_dir.y, horiz_len);
    blaster.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
}

fn switch_blaster(
    mut selection: ResMut<BlasterSelection>,
    car_query: Query<Entity, With<PlayerCar>>,
    blaster_query: Query<Entity, With<BlasterVisual>>,
    children_query: Query<&Children>,
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

    let def = &BLASTER_DEFS[selection.index];
    let scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(def.path));
    commands.entity(car_entity).with_children(|parent| {
        parent.spawn((
            SceneRoot(scene),
            Transform::from_translation(Vec3::new(0.0, 1.2, 0.0)).with_scale(Vec3::splat(2.0)),
            BlasterVisual,
        ));
    });
}
