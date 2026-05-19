use bevy::prelude::*;
use avian3d::prelude::*;

pub struct CarPlugin;

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CarParams>()
            .init_resource::<Telemetry>()
            .init_resource::<WheelState>()
            .init_resource::<CarState>()
            .init_resource::<SkidOffsets>()
            .init_resource::<GroundInfo>()
            .add_systems(Startup, setup_skid_assets)
            .add_systems(PreUpdate, (ground_raycast, car_movement).chain())
            .add_systems(Update, (camera_follow, label_wheels, animate_wheels, record_telemetry, spawn_skid_marks, fade_skid_marks));
    }
}

#[derive(Component)]
pub struct Car {
    pub speed: f32,
    pub yaw: f32,
    pub y_velocity: f32,
    pub airborne: bool,
}

#[derive(Component)]
pub struct PlayerCar;

#[derive(Component)]
pub struct CarCamera;

#[derive(Component)]
pub struct MinimapCamera;

#[derive(Component)]
pub struct CarVisual;

#[derive(Component)]
pub struct WheelFrontLeft;

#[derive(Component)]
pub struct WheelFrontRight;

#[derive(Component)]
pub struct WheelRearLeft;

#[derive(Component)]
pub struct WheelRearRight;

#[derive(Component)]
pub struct WheelsLabeled;

pub const GRAVITY: f32 = 20.0;
pub const JUMP_IMPULSE: f32 = 14.0;
pub const GROUND_RAY_DISTANCE: f32 = 100.0;
pub const CAR_GROUND_OFFSET: f32 = 0.05;
pub const ARENA_RADIUS: f32 = 250.0;
pub const AIRBORNE_THRESHOLD: f32 = 1.5;
pub const SLOPE_ACCEL_FACTOR: f32 = 30.0;

#[derive(Resource, Default)]
pub struct GroundInfo {
    pub y: f32,
    pub normal: Vec3,
}

#[derive(Resource)]
pub struct Telemetry {
    pub speed_history: Vec<f32>,
    pub steer_history: Vec<f32>,
    pub yaw_rate_history: Vec<f32>,
    max_samples: usize,
}

impl Default for Telemetry {
    fn default() -> Self {
        Self {
            speed_history: Vec::new(),
            steer_history: Vec::new(),
            yaw_rate_history: Vec::new(),
            max_samples: 300,
        }
    }
}

impl Telemetry {
    pub fn record(&mut self, speed: f32, steer: f32, yaw_rate: f32) {
        self.speed_history.push(speed);
        self.steer_history.push(steer);
        self.yaw_rate_history.push(yaw_rate);
        if self.speed_history.len() > self.max_samples {
            self.speed_history.remove(0);
            self.steer_history.remove(0);
            self.yaw_rate_history.remove(0);
        }
    }
}

#[derive(Resource)]
pub struct CarParams {
    pub acceleration: f32,
    pub max_speed: f32,
    pub friction: f32,
    pub brake_force: f32,
    pub steer_rate: f32,
}

impl Default for CarParams {
    fn default() -> Self {
        Self {
            acceleration: 60.0,
            max_speed: 80.0,
            friction: 0.3,
            brake_force: 45.0,
            steer_rate: 2.5,
        }
    }
}

#[derive(Resource, Default)]
pub struct WheelState {
    pub current_angle: f32,
    pub target_angle: f32,
}

#[derive(Resource, Default)]
pub struct CarState {
    pub speed: f32,
    pub yaw: f32,
    pub position: Vec3,
    pub braking: bool,
    pub skidding: bool,
    pub boosting: bool,
    pub prev_speed: f32,
}

#[derive(Component)]
pub struct SkidMark {
    pub timer: Timer,
}

#[derive(Resource)]
pub struct SkidMarkAssets {
    pub mesh: Handle<Mesh>,
}

#[derive(Resource)]
pub struct SkidOffsets {
    pub left: f32,
    pub right: f32,
}

impl Default for SkidOffsets {
    fn default() -> Self {
        Self { left: -0.07, right: -0.62 }
    }
}

fn ground_raycast(
    spatial_query: SpatialQuery,
    car_query: Query<(Entity, &Position), With<PlayerCar>>,
    mut ground_info: ResMut<GroundInfo>,
) {
    let Ok((car_entity, car_pos)) = car_query.single() else { return };
    let ray_origin = car_pos.0 + Vec3::new(0.0, GROUND_RAY_DISTANCE, 0.0);
    let filter = SpatialQueryFilter::from_excluded_entities([car_entity]);
    let ground_hit = spatial_query.cast_ray(
        ray_origin,
        Dir3::NEG_Y,
        GROUND_RAY_DISTANCE * 2.0,
        true,
        &filter,
    );
    match ground_hit {
        Some(hit) => {
            ground_info.y = ray_origin.y - hit.distance;
            ground_info.normal = hit.normal;
        }
        None => {
            ground_info.y = 0.0;
            ground_info.normal = Vec3::Y;
        }
    }
}

fn car_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    params: Res<CarParams>,
    ground_info: Res<GroundInfo>,
    mut query: Query<(&mut Car, &mut Position, &mut Rotation), With<PlayerCar>>,
    mut car_state: ResMut<CarState>,
) {
    let dt = time.delta_secs();

    let throttle = if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        1.0
    } else if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        -0.5
    } else {
        0.0
    };

    let steer_input = if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        1.0
    } else if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        -1.0
    } else {
        0.0
    };

    let braking = keys.pressed(KeyCode::Space);
    let boosting = keys.pressed(KeyCode::ShiftLeft);
    let jumping = keys.just_pressed(KeyCode::ShiftRight);

    let (mut car, mut position, mut rotation) = match query.single_mut() {
        Ok(q) => q,
        Err(_) => return,
    };

    let boost_multiplier = if boosting { 1.5 } else { 1.0 };
    let steer_penalty = if boosting { 0.5 } else { 1.0 };
    let air_steer = if car.airborne { 0.3 } else { 1.0 };
    let max_speed_boosted = params.max_speed * boost_multiplier;
    let steer = steer_input * params.steer_rate * steer_penalty * air_steer * (1.0 - (car.speed / max_speed_boosted).abs() * 0.5);

    if jumping && !car.airborne {
        car.y_velocity = JUMP_IMPULSE;
        car.airborne = true;
    }

    if car.airborne {
        car.y_velocity -= GRAVITY * dt;
    }

    if braking {
        let brake_amount = params.brake_force * dt;
        if car.speed > 0.0 {
            car.speed = (car.speed - brake_amount).max(0.0);
        } else if car.speed < 0.0 {
            car.speed = (car.speed + brake_amount).min(0.0);
        }
    } else {
        let accel = throttle * params.acceleration * boost_multiplier;
        car.speed += (accel - car.speed * params.friction) * dt;
    }

    let steer_friction = if braking {
        steer.abs() * car.speed.abs() * 0.02
    } else {
        steer.abs() * car.speed.abs() * 0.10
    };
    car.speed -= car.speed.signum() * steer_friction * dt;

    if !car.airborne {
        let slope_forward = -(ground_info.normal.x * car.yaw.sin() + ground_info.normal.z * car.yaw.cos());
        car.speed += slope_forward * SLOPE_ACCEL_FACTOR * dt;
    }

    car.speed = car.speed.clamp(-params.max_speed * 0.3, max_speed_boosted);
    car.yaw += steer * dt * (car.speed / 30.0).clamp(-1.0, 1.0);

    let forward = Vec3::new(car.yaw.sin(), 0.0, car.yaw.cos());
    let mut new_xz = position.0 + forward * car.speed * dt;

    let dist = (new_xz.x * new_xz.x + new_xz.z * new_xz.z).sqrt();
    if dist > ARENA_RADIUS {
        let scale = ARENA_RADIUS / dist;
        new_xz.x *= scale;
        new_xz.z *= scale;
        car.speed *= 0.5;
    }

    position.0.x = new_xz.x;
    position.0.z = new_xz.z;

    let target_y = ground_info.y + CAR_GROUND_OFFSET;

    if car.airborne || car.y_velocity > 0.0 {
        position.0.y += car.y_velocity * dt;
        car.y_velocity -= GRAVITY * dt;
        if position.0.y <= target_y && car.y_velocity <= 0.0 {
            position.0.y = target_y;
            car.y_velocity = 0.0;
            car.airborne = false;
        }
    } else if position.0.y > target_y + AIRBORNE_THRESHOLD {
        car.airborne = true;
        car.y_velocity = 0.0;
    } else {
        position.0.y = position.0.y.lerp(target_y, 0.4);
        car.airborne = false;
    }

    if !car.airborne && position.0.y < target_y {
        position.0.y = target_y;
    }

    let yaw_quat = Quat::from_rotation_y(car.yaw);
    if car.airborne {
        *rotation = Rotation::from(yaw_quat);
    } else {
        *rotation = Rotation::from(align_to_ground(yaw_quat, ground_info.normal));
    }

    let decel_rate = (car.speed / params.max_speed).abs() * params.friction;
    let speed_delta = (car.speed - car_state.prev_speed).abs() / dt.max(0.001);
    car_state.skidding = (braking && car.speed.abs() > 10.0) || (throttle == 0.0 && car.speed.abs() > 40.0 && decel_rate > 1.5) || (speed_delta > 25.0 && car.speed.abs() > 10.0);
    car_state.prev_speed = car.speed;
    car_state.speed = car.speed;
    car_state.yaw = car.yaw;
    car_state.braking = braking;
    car_state.boosting = boosting;
    car_state.position = position.0;
}

fn align_to_ground(yaw_quat: Quat, normal: Vec3) -> Quat {
    let forward = yaw_quat * Vec3::NEG_Z;
    let projected_forward = (forward - normal * forward.dot(normal)).normalize_or_zero();
    if projected_forward.is_finite() && projected_forward.length_squared() > 0.001 {
        let right = normal.cross(projected_forward).normalize_or_zero();
        if right.is_finite() && right.length_squared() > 0.001 {
            Quat::from_mat3(&Mat3::from_cols(right, normal, projected_forward))
        } else {
            yaw_quat
        }
    } else {
        yaw_quat
    }
}

fn camera_follow(
    car_query: Query<(&Car, &Position), With<PlayerCar>>,
    mut cam_query: Query<&mut Transform, (With<CarCamera>, Without<PlayerCar>)>,
) {
    let Some((car, car_pos)) = car_query.iter().next() else {
        return;
    };

    let car_pos = car_pos.0;
    let behind = Vec3::new(car.yaw.sin(), 0.0, car.yaw.cos()) * -8.0;
    let target = car_pos + behind + Vec3::new(0.0, 5.0, 0.0);

    for mut cam in cam_query.iter_mut() {
        cam.translation = cam.translation.lerp(target, 0.05);
        cam.look_at(car_pos + Vec3::new(0.0, 1.0, 0.0), Vec3::Y);
    }
}

fn label_wheels(
    car_query: Query<Entity, (With<PlayerCar>, Without<WheelsLabeled>)>,
    children_query: Query<&Children>,
    name_query: Query<&Name>,
    mut commands: Commands,
) {
    for car_entity in car_query.iter() {
        let mut found_wheels = false;
        for child in children_query.iter_descendants(car_entity) {
            if let Ok(name) = name_query.get(child) {
                match name.as_str() {
                    "wheelFrontLeft" => {
                        commands.entity(child).insert(WheelFrontLeft);
                        found_wheels = true;
                    }
                    "wheelFrontRight" => {
                        commands.entity(child).insert(WheelFrontRight);
                        found_wheels = true;
                    }
                    "wheelBackLeft" => {
                        commands.entity(child).insert(WheelRearLeft);
                        found_wheels = true;
                    }
                    "wheelBackRight" => {
                        commands.entity(child).insert(WheelRearRight);
                        found_wheels = true;
                    }
                    _ => {}
                }
            }
        }
        if found_wheels {
            commands.entity(car_entity).insert(WheelsLabeled);
        }
    }
}

fn animate_wheels(
    time: Res<Time>,
    car_data: Query<&Car, With<PlayerCar>>,
    params: Res<CarParams>,
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel_state: ResMut<WheelState>,
    mut front_left: Query<&mut Transform, (With<WheelFrontLeft>, Without<WheelFrontRight>, Without<WheelRearLeft>, Without<WheelRearRight>, Without<PlayerCar>)>,
    mut front_right: Query<&mut Transform, (With<WheelFrontRight>, Without<WheelFrontLeft>, Without<WheelRearLeft>, Without<WheelRearRight>, Without<PlayerCar>)>,
) {
    let Some(car) = car_data.iter().next() else {
        return;
    };
    let dt = time.delta_secs();

    let steer_input = if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        1.0
    } else if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        -1.0
    } else {
        0.0
    };

    let effective_steer = steer_input * params.steer_rate * (1.0 - (car.speed / params.max_speed).abs() * 0.5);
    wheel_state.target_angle = effective_steer * 0.3;
    let smoothing = 1.0 - (-8.0 * dt).exp();
    wheel_state.current_angle += (wheel_state.target_angle - wheel_state.current_angle) * smoothing;

    let steer_rot = Quat::from_rotation_y(wheel_state.current_angle);

    for mut t in front_left.iter_mut() {
        t.rotation = steer_rot;
    }
    for mut t in front_right.iter_mut() {
        t.rotation = steer_rot;
    }
}

fn record_telemetry(
    mut telemetry: ResMut<Telemetry>,
    wheel_state: Res<WheelState>,
    car_query: Query<&Car, With<PlayerCar>>,
) {
    let Ok(car) = car_query.single() else {
        return;
    };
    let yaw_rate = car.yaw;
    telemetry.record(car.speed, wheel_state.current_angle, yaw_rate);
}

fn setup_skid_assets(
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    let mesh = meshes.add(Rectangle::new(0.35, 0.9));
    commands.insert_resource(SkidMarkAssets { mesh });
}

fn spawn_skid_marks(
    time: Res<Time>,
    car_state: Res<CarState>,
    ground_info: Res<GroundInfo>,
    skid_assets: Res<SkidMarkAssets>,
    skid_offsets: Res<SkidOffsets>,
    skid_count: Query<(), With<SkidMark>>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cooldown: Local<f32>,
) {
    *cooldown = (*cooldown - time.delta_secs()).max(0.0);

    if !car_state.skidding || car_state.speed.abs() < 10.0 || *cooldown > 0.0 {
        return;
    }

    if skid_count.iter().count() > 600 {
        return;
    }

    *cooldown = 0.02;

    let speed_ratio = ((car_state.speed.abs() - 15.0) / 80.0).min(1.0);
    let base_alpha = 0.4 + 0.4 * speed_ratio;

    let forward = Vec3::new(car_state.yaw.sin(), 0.0, car_state.yaw.cos());
    let right = Vec3::new(car_state.yaw.cos(), 0.0, -car_state.yaw.sin());
    let yaw_quat = Quat::from_rotation_y(car_state.yaw);
    let normal = ground_info.normal;
    let surface_align = Quat::from_rotation_arc(Vec3::Y, normal);
    let rotation = surface_align * yaw_quat * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    let skid_y = ground_info.y + 0.02;

    for lateral in [skid_offsets.left, skid_offsets.right] {
        let pos = car_state.position - forward * 1.0 + right * lateral;
        let pos = Vec3::new(pos.x, skid_y, pos.z);

        commands.spawn((
            Mesh3d(skid_assets.mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.08, 0.08, 0.08, base_alpha),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(pos).with_rotation(rotation),
            SkidMark {
                timer: Timer::from_seconds(2.0, TimerMode::Once),
            },
        ));
    }
}

fn fade_skid_marks(
    time: Res<Time>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<(Entity, &mut SkidMark, &MeshMaterial3d<StandardMaterial>)>,
) {
    for (entity, mut skid, mat) in query.iter_mut() {
        skid.timer.tick(time.delta());
        let ratio = skid.timer.fraction();
        if let Some(material) = materials.get_mut(&mat.0) {
            material.base_color.set_alpha((1.0 - ratio) * 0.8);
        }
        if skid.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}