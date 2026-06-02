use avian3d::prelude::*;
use bevy::prelude::*;

pub struct CarPlugin;

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehiclePhysicsConfig>()
            .init_resource::<WheelState>()
            .init_resource::<CarInput>()
            .init_resource::<CarState>()
            .init_resource::<Telemetry>()
            .init_resource::<GroundState>()
            .add_systems(
                FixedPostUpdate,
                (ground_detection_system, apply_vehicle_forces, smooth_angular_velocity).chain().in_set(PhysicsSystems::Prepare),
            )
            .add_systems(
                Update,
                (
                    capture_input,
                    sync_car_state,
                    camera_follow,
                    label_wheels,
                    animate_wheels,
                    record_telemetry,
                    respawn_car,
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

#[derive(Component, Debug, Clone)]
pub struct VehicleData {
    pub lateral_velocity: f32,
    pub lateral_force: f32,
    pub slip_ratio: f32,
    pub grip_state: GripState,
    pub surface_normal: Vec3,
    pub grounded: bool,
    pub forward_speed: f32,
}

impl Default for VehicleData {
    fn default() -> Self {
        Self {
            lateral_velocity: 0.0,
            lateral_force: 0.0,
            slip_ratio: 0.0,
            grip_state: GripState::Static,
            surface_normal: Vec3::Y,
            grounded: false,
            forward_speed: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GripState {
    Static,
    Kinetic,
}

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

#[derive(Resource)]
pub struct VehiclePhysicsConfig {
    pub downforce: f32,
    pub downforce_speed: f32,
    pub lateral_stiffness: f32,
    pub slip_threshold: f32,
    pub kinetic_friction: f32,
    pub engine_force: f32,
    pub brake_force: f32,
    pub steer_torque: f32,
    pub roll_torque: f32,
    pub rolling_resistance: f32,
    pub drag_coefficient: f32,
    pub max_speed: f32,
    pub steer_smoothing: f32,
    pub max_steer_angle: f32,
    pub steer_speed_response: f32,
}

impl Default for VehiclePhysicsConfig {
    fn default() -> Self {
        Self {
            downforce: 80.0,
            downforce_speed: 0.02,
            lateral_stiffness: 300.0,
            slip_threshold: 4.0,
            kinetic_friction: 8000.0,
            engine_force: 6000.0,
            brake_force: 15000.0,
            steer_torque: 600.0,
    roll_torque: 300.0,
            rolling_resistance: 3.0,
            drag_coefficient: 0.4,
            max_speed: 40.0,
            steer_smoothing: 10.0,
            max_steer_angle: 0.45,
            steer_speed_response: 25.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct WheelState {
    pub current_angle: f32,
    pub target_angle: f32,
}

#[derive(Resource, Default)]
pub struct CarInput {
    pub throttle: f32,
    pub steer: f32,
    pub braking: bool,
    pub boosting: bool,
    pub roll: f32,
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
pub struct GroundState {
    pub grounded: bool,
    pub surface_normal: Vec3,
    pub raw_normal: Vec3,
    normal_smooth: Vec3,
}

impl Default for GroundState {
    fn default() -> Self {
        Self {
            grounded: false,
            surface_normal: Vec3::Y,
            raw_normal: Vec3::Y,
            normal_smooth: Vec3::Y,
        }
    }
}

fn ground_detection_system(
    collisions: Collisions,
    car_query: Query<Entity, With<PlayerCar>>,
    car_rot_query: Query<&Rotation, With<PlayerCar>>,
    mut ground_state: ResMut<GroundState>,
) {
    let Ok(car_entity) = car_query.single() else {
        return;
    };
    let Ok(car_rotation) = car_rot_query.single() else {
        return;
    };
    let car_down = car_rotation.0 * Vec3::NEG_Y;

    let mut grounded = false;
    let mut weighted_normal = Vec3::ZERO;
    let mut total_weight = 0.0_f32;

    for contact_pair in collisions.iter() {
        let is_car = contact_pair.body1 == Some(car_entity)
            || contact_pair.body2 == Some(car_entity);

        if !is_car {
            continue;
        }

        for manifold in &contact_pair.manifolds {
            let raw_normal = if contact_pair.body1 == Some(car_entity) {
                -manifold.normal
            } else {
                manifold.normal
            };

            let facing_car_bottom = car_down.dot(raw_normal) < 0.0;
            if !facing_car_bottom {
                continue;
            }

            for point in &manifold.points {
                if point.penetration < 0.0 {
                    continue;
                }
                let weight = point.penetration;
                weighted_normal += raw_normal * weight;
                total_weight += weight;
                grounded = true;
            }
        }
    }

    let raw_normal = if total_weight > 0.0 {
        weighted_normal.normalize_or(Vec3::Y)
    } else {
        Vec3::Y
    };

    let smooth = 0.15;
    ground_state.normal_smooth = ground_state.normal_smooth.lerp(raw_normal, smooth);
    let smoothed = ground_state.normal_smooth.normalize_or(Vec3::Y);

    ground_state.grounded = grounded;
    ground_state.raw_normal = raw_normal;
    ground_state.surface_normal = smoothed;
}

fn apply_vehicle_forces(
    input: Res<CarInput>,
    config: Res<VehiclePhysicsConfig>,
    ground: Res<GroundState>,
    mut forces_query: Query<Forces, With<PlayerCar>>,
    mut data_query: Query<&mut VehicleData, With<PlayerCar>>,
) {
    let Ok(mut forces) = forces_query.single_mut() else {
        return;
    };
    let Ok(mut vehicle_data) = data_query.single_mut() else {
        return;
    };

    let car_rot = forces.rotation().0;
    let velocity = forces.linear_velocity();
    let speed = velocity.length();
    let forward = car_rot * Vec3::Z;
    let right = car_rot * Vec3::X;
    let forward_speed = velocity.dot(forward);

    let grounded = ground.grounded;
    let surface_normal = ground.surface_normal;
    let raw_normal = ground.raw_normal;
    let car_down = car_rot * Vec3::NEG_Y;
    let bottom_contact = car_down.dot(raw_normal) < -0.7;

    forces.apply_force(-Vec3::Y * config.downforce);
    forces.apply_force(-Vec3::Y * config.downforce * config.downforce_speed * speed);

    let lateral_vel = velocity.dot(right);

    if bottom_contact {
        let abs_lateral = lateral_vel.abs();
        let slip_ratio = abs_lateral / config.slip_threshold;

        let lateral_force_mag = if abs_lateral < config.slip_threshold {
            vehicle_data.grip_state = GripState::Static;
            (config.lateral_stiffness * abs_lateral).min(config.lateral_stiffness * config.slip_threshold)
        } else {
            vehicle_data.grip_state = GripState::Kinetic;
            let peak = config.lateral_stiffness * config.slip_threshold;
            let excess = slip_ratio - 1.0;
            peak / (1.0 + excess) + config.kinetic_friction * excess / (1.0 + excess)
        };

        forces.apply_force(-right * lateral_vel.signum() * lateral_force_mag);
        vehicle_data.lateral_force = lateral_force_mag;
    } else {
        vehicle_data.grip_state = GripState::Kinetic;
        vehicle_data.lateral_force = 0.0;
    }

    vehicle_data.lateral_velocity = lateral_vel;
    vehicle_data.slip_ratio = lateral_vel.abs() / config.slip_threshold;
    vehicle_data.surface_normal = surface_normal;
    vehicle_data.grounded = grounded;
    vehicle_data.forward_speed = forward_speed;

    let boost = if input.boosting { 1.5 } else { 1.0 };

    if bottom_contact {
        let surface_forward = (forward - forward.dot(surface_normal) * surface_normal)
            .normalize_or(forward);
        forces.apply_force(surface_forward * input.throttle * config.engine_force * boost);
    }

    if input.braking && bottom_contact {
        forces.apply_force(-velocity.normalize_or(Vec3::ZERO) * config.brake_force);
    }

    let steer_strength = (forward_speed.abs() / config.steer_speed_response)
        .min(1.0)
        .max(0.0);
    if steer_strength > 0.01 && bottom_contact {
        forces.apply_torque(car_rot * Vec3::Y * input.steer * config.steer_torque * steer_strength);
    }

    if input.roll != 0.0 {
        forces.apply_torque(car_rot * Vec3::Z * input.roll * config.roll_torque);
    }

    forces.apply_force(-velocity * config.rolling_resistance);
    forces.apply_force(-velocity * speed * config.drag_coefficient);

    if speed > config.max_speed {
        let excess_drag = config.drag_coefficient + 2.0 * (speed - config.max_speed);
        forces.apply_force(-velocity * excess_drag);
    }
}

fn smooth_angular_velocity(
    ground: Res<GroundState>,
    car_rot_query: Query<&Rotation, With<PlayerCar>>,
    mut query: Query<&mut AngularVelocity, With<PlayerCar>>,
) {
    let Ok(car_rotation) = car_rot_query.single() else {
        return;
    };
    let Ok(mut ang_vel) = query.single_mut() else {
        return;
    };
    let car_down = car_rotation.0 * Vec3::NEG_Y;
    if !ground.grounded || car_down.dot(ground.raw_normal) >= -0.7 {
        return;
    }
    ang_vel.0 *= 0.6;
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

    input.roll = if keys.pressed(KeyCode::KeyQ) {
        -1.0
    } else if keys.pressed(KeyCode::KeyE) {
        1.0
    } else {
        0.0
    };
}

fn sync_car_state(
    input: Res<CarInput>,
    mut car_query: Query<
        (&LinearVelocity, &Rotation, &Position, &mut Car),
        With<PlayerCar>,
    >,
    vehicle_data: Query<&VehicleData, With<PlayerCar>>,
    mut car_state: ResMut<CarState>,
) {
    let Ok((lin_vel, rotation, position, mut car)) = car_query.single_mut() else {
        return;
    };

    let forward = rotation.0 * Vec3::Z;
    let forward_speed = lin_vel.0.dot(forward);
    let yaw = forward.x.atan2(forward.z);

    car.speed = forward_speed;
    car.yaw = yaw;

    let is_drifting = vehicle_data
        .iter()
        .next()
        .map_or(false, |d| d.grip_state == GripState::Kinetic);

    car_state.prev_speed = car_state.speed;
    car_state.speed = forward_speed;
    car_state.yaw = yaw;
    car_state.position = position.0;
    car_state.braking = input.braking;
    car_state.boosting = input.boosting;
    car_state.skidding = input.braking || is_drifting;
}

fn camera_follow(
    car_query: Query<(&Rotation, &Position), With<PlayerCar>>,
    mut cam_query: Query<&mut Transform, (With<CarCamera>, Without<PlayerCar>)>,
) {
    let Some((car_rot, car_pos)) = car_query.iter().next() else {
        return;
    };

    let facing = *car_rot * Vec3::Z;
    let flat = Vec3::new(facing.x, 0.0, facing.z).normalize_or(Vec3::Z);
    let target = car_pos.0 - flat * 8.0 + Vec3::new(0.0, 5.0, 0.0);

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
    config: Res<VehiclePhysicsConfig>,
    mut wheel_state: ResMut<WheelState>,
    mut front_left: Query<
        &mut Transform,
        (
            With<WheelFrontLeft>,
            Without<WheelFrontRight>,
            Without<WheelRearLeft>,
            Without<WheelRearRight>,
            Without<PlayerCar>,
        ),
    >,
    mut front_right: Query<
        &mut Transform,
        (
            With<WheelFrontRight>,
            Without<WheelFrontLeft>,
            Without<WheelRearLeft>,
            Without<WheelRearRight>,
            Without<PlayerCar>,
        ),
    >,
    _rear_left: Query<
        &mut Transform,
        (
            With<WheelRearLeft>,
            Without<WheelFrontLeft>,
            Without<WheelFrontRight>,
            Without<WheelRearRight>,
            Without<PlayerCar>,
        ),
    >,
    _rear_right: Query<
        &mut Transform,
        (
            With<WheelRearRight>,
            Without<WheelFrontLeft>,
            Without<WheelFrontRight>,
            Without<WheelRearLeft>,
            Without<PlayerCar>,
        ),
    >,
) {
    let Some(_car) = car_data.iter().next() else {
        return;
    };
    let dt = time.delta_secs();

    let skid_mult = if car_state.skidding { 0.4 } else { 1.0 };
    let effective_steer = input.steer
        * skid_mult
        * (1.0 - (car_state.speed / 30.0).abs().clamp(0.0, 1.0) * 0.5);
    wheel_state.target_angle = effective_steer * config.max_steer_angle;
    let smoothing = 1.0 - (-config.steer_smoothing * dt).exp();
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

fn respawn_car(
    mut car_query: Query<(&mut Position, &mut LinearVelocity, &mut AngularVelocity), With<PlayerCar>>,
) {
    let Ok((mut pos, mut lin_vel, mut ang_vel)) = car_query.single_mut() else {
        return;
    };
    if pos.0.y < -20.0 {
        pos.0 = Vec3::new(0.0, 5.0, 0.0);
        lin_vel.0 = Vec3::ZERO;
        ang_vel.0 = Vec3::ZERO;
    }
}

