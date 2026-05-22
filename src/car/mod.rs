use bevy::prelude::*;
use avian3d::prelude::*;

pub struct CarPlugin;

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CarParams>()
            .init_resource::<Telemetry>()
            .init_resource::<WheelState>()
            .init_resource::<CarState>()
            .init_resource::<CarInput>()
            .init_resource::<GroundInfo>()
            .init_resource::<CarColliderEntities>()
            .add_systems(PreUpdate, (
                collect_car_colliders,
                ground_raycast,
                car_movement,
            ).chain())
            .add_systems(
                Update,
                (
                    capture_input,
                    camera_follow,
                    label_wheels,
                    animate_wheels,
                    record_telemetry,
                    debug_log,
                ),
            );
    }
}

#[derive(Component)]
pub struct Car {
    pub speed: f32,
    pub yaw: f32,
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

const GROUND_RAY_DISTANCE: f32 = 100.0;
const CAR_GROUND_OFFSET: f32 = 0.02;
const SLOPE_ACCEL_FACTOR: f32 = 30.0;

#[derive(Resource, Default)]
pub struct GroundInfo {
    pub y: f32,
    pub normal: Vec3,
}

#[derive(Resource, Default)]
pub struct CarInput {
    pub throttle: f32,
    pub steer: f32,
    pub braking: bool,
    pub boosting: bool,
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
    pub boosting: bool,
    pub skidding: bool,
    pub prev_speed: f32,
}

#[derive(Resource, Default)]
pub struct CarColliderEntities(pub Vec<Entity>);

fn collect_car_colliders(
    car_query: Query<Entity, With<PlayerCar>>,
    children_query: Query<&Children>,
    mut colliders: ResMut<CarColliderEntities>,
) {
    // Skip if already fully populated (car entity + at least child entity found)
    if colliders.0.len() > 1 {
        return;
    }
    let Ok(car_entity) = car_query.single() else {
        return;
    };
    let old_len = colliders.0.len();
    let mut all = vec![car_entity];
    collect_descendants(&children_query, car_entity, &mut all);
    colliders.0 = all;
    // Only log on first successful collection (when children were found)
    if colliders.0.len() > 1 && old_len <= 1 {
        info!("car collider entities collected: {} total", colliders.0.len());
    }
}

fn collect_descendants(children_query: &Query<&Children>, entity: Entity, out: &mut Vec<Entity>) {
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            out.push(child);
            collect_descendants(children_query, child, out);
        }
    }
}

fn capture_input(keys: Res<ButtonInput<KeyCode>>, mut input: ResMut<CarInput>) {
    input.throttle = if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        1.0
    } else if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        -0.5
    } else {
        0.0
    };

    input.steer = if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        1.0
    } else if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        -1.0
    } else {
        0.0
    };

    input.braking = keys.pressed(KeyCode::Space);
    input.boosting = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
}

fn ground_raycast(
    spatial_query: SpatialQuery,
    car_query: Query<(Entity, &Position), With<PlayerCar>>,
    colliders: Res<CarColliderEntities>,
    mut ground_info: ResMut<GroundInfo>,
) {
    let Ok((_car_entity, car_pos)) = car_query.single() else {
        return;
    };
    let ray_origin = car_pos.0 + Vec3::new(0.0, GROUND_RAY_DISTANCE, 0.0);
    let filter = SpatialQueryFilter::from_excluded_entities(colliders.0.iter().copied());
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
    input: Res<CarInput>,
    params: Res<CarParams>,
    ground_info: Res<GroundInfo>,
    mut query: Query<(&mut Car, &mut Position, &mut Rotation), With<PlayerCar>>,
    mut car_state: ResMut<CarState>,
) {
    let dt = time.delta_secs();
    let Ok((mut car, mut position, mut rotation)) = query.single_mut() else {
        return;
    };

    let boost_multiplier = if input.boosting { 1.5 } else { 1.0 };
    let max_speed_boosted = params.max_speed * boost_multiplier;
    let handbrake_turn = input.braking
        && input.throttle > 0.0
        && input.steer.abs() > 0.1
        && car.speed > 5.0;
    let handbrake_boost = if handbrake_turn { 1.4 } else { 1.0 };
    let steer = input.steer
        * params.steer_rate
        * handbrake_boost
        * (1.0 - (car.speed / max_speed_boosted).abs() * 0.5);

    if input.braking {
        let mut brake_amount = params.brake_force * dt;
        if handbrake_turn {
            brake_amount *= 0.45;
        }
        if car.speed > 0.0 {
            car.speed = (car.speed - brake_amount).max(0.0);
        } else if car.speed < 0.0 {
            car.speed = (car.speed + brake_amount).min(0.0);
        }
    } else {
        let accel = input.throttle * params.acceleration * boost_multiplier;
        car.speed += (accel - car.speed * params.friction) * dt;
    }

    let steer_friction = if input.braking {
        steer.abs() * car.speed.abs() * 0.02
    } else {
        steer.abs() * car.speed.abs() * 0.10
    };
    car.speed -= car.speed.signum() * steer_friction * dt;

    let slope_forward = -(ground_info.normal.x * car.yaw.sin() + ground_info.normal.z * car.yaw.cos());
    car.speed += slope_forward * SLOPE_ACCEL_FACTOR * dt;

    car.speed = car.speed.clamp(-params.max_speed * 0.3, max_speed_boosted);
    car.yaw += steer * dt * (car.speed / 30.0).clamp(-1.0, 1.0);

    let forward = Vec3::new(car.yaw.sin(), 0.0, car.yaw.cos());
    position.0 += forward * car.speed * dt;

    let target_y = ground_info.y + CAR_GROUND_OFFSET;
    position.0.y = target_y;

    let yaw_quat = Quat::from_rotation_y(car.yaw);
    *rotation = Rotation::from(align_to_ground(yaw_quat, ground_info.normal));

    let decel_rate = (car.speed / params.max_speed).abs() * params.friction;
    let speed_delta = (car.speed - car_state.prev_speed).abs() / dt.max(0.001);
    car_state.skidding = handbrake_turn
        || (input.braking && car.speed.abs() > 10.0)
        || (input.throttle == 0.0 && car.speed.abs() > 40.0 && decel_rate > 1.5)
        || (speed_delta > 25.0 && car.speed.abs() > 10.0);
    car_state.prev_speed = car.speed;
    car_state.speed = car.speed;
    car_state.yaw = car.yaw;
    car_state.braking = input.braking;
    car_state.boosting = input.boosting;
    car_state.position = position.0;
}

fn align_to_ground(yaw_quat: Quat, normal: Vec3) -> Quat {
    let n = normal.normalize_or_zero();
    if n.length_squared() < 0.001 || n.dot(Vec3::Y) > 0.9999 {
        return yaw_quat;
    }
    let forward = yaw_quat * Vec3::Z;
    let right = n.cross(forward).normalize_or_zero();
    if right.is_finite() && right.length_squared() > 0.001 {
        let corrected_forward = right.cross(n);
        Quat::from_mat3(&Mat3::from_cols(right, n, corrected_forward))
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

    let forward = Vec3::new(car.yaw.sin(), 0.0, car.yaw.cos());
    let target = car_pos.0 - forward * 8.0 + Vec3::new(0.0, 5.0, 0.0);

    for mut cam in cam_query.iter_mut() {
        cam.translation = cam.translation.lerp(target, 0.05);
        cam.look_at(car_pos.0 + Vec3::new(0.0, 1.0, 0.0), Vec3::Y);
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
    input: Res<CarInput>,
    car_state: Res<CarState>,
    mut wheel_state: ResMut<WheelState>,
    mut front_left: Query<&mut Transform, (With<WheelFrontLeft>, Without<WheelFrontRight>, Without<WheelRearLeft>, Without<WheelRearRight>, Without<PlayerCar>)>,
    mut front_right: Query<&mut Transform, (With<WheelFrontRight>, Without<WheelFrontLeft>, Without<WheelRearLeft>, Without<WheelRearRight>, Without<PlayerCar>)>,
    _rear_left: Query<&mut Transform, (With<WheelRearLeft>, Without<WheelFrontLeft>, Without<WheelFrontRight>, Without<WheelRearRight>, Without<PlayerCar>)>,
    _rear_right: Query<&mut Transform, (With<WheelRearRight>, Without<WheelFrontLeft>, Without<WheelFrontRight>, Without<WheelRearLeft>, Without<PlayerCar>)>,
) {
    let Some(_car) = car_data.iter().next() else {
        return;
    };
    let dt = time.delta_secs();

    let skid_mult = if car_state.skidding { 0.4 } else { 1.0 };
    let effective_steer = input.steer * skid_mult * (1.0 - (car_state.speed / 30.0).abs().clamp(0.0, 1.0) * 0.5);
    wheel_state.target_angle = effective_steer * 0.45;
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
    telemetry.record(car.speed, wheel_state.current_angle, car.yaw);
}

fn debug_log(
    mut frame: Local<u32>,
    input: Res<CarInput>,
    car_state: Res<CarState>,
    car_query: Query<(&Position, &Rotation, &Car), With<PlayerCar>>,
) {
    *frame += 1;
    if *frame % 30 != 0 {
        return;
    }
    use std::f32::consts::PI;
    let Ok((pos, rot, car)) = car_query.single() else {
        return;
    };
    let (roll, yaw, pitch) = rot.0.to_euler(EulerRot::ZYX);
    info!(
        "\n─── frame={} ───\n\
         input: t={:.2} s={:.2} brake={} boost={}\n\
         pos: ({:.2}, {:.2}, {:.2})  yaw={:.1}° pitch={:.1}° roll={:.1}°\n\
         speed={:.1}  skid={}",
        *frame,
        input.throttle, input.steer, input.braking, input.boosting,
        pos.0.x, pos.0.y, pos.0.z, yaw * 180.0 / PI, pitch * 180.0 / PI, roll * 180.0 / PI,
        car.speed, car_state.skidding,
    );
}
