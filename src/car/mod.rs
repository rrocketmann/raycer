use std::f32::consts::PI;

use avian3d::prelude::*;
use bevy::prelude::*;

pub struct CarPlugin;

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehiclePhysicsConfig>()
            .init_resource::<SuspensionState>()
            .init_resource::<WheelState>()
            .init_resource::<CarInput>()
            .init_resource::<CarColliderEntities>()
            .init_resource::<CarState>()
            .init_resource::<Telemetry>()
            .add_systems(
                FixedPostUpdate,
                (
                    raycast_suspension_system,
                    surface_alignment_system,
                    apply_vehicle_forces,
                )
                    .chain()
                    .in_set(PhysicsSystems::Prepare),
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
                ),
            );
    }
}

// ─── Components ──────────────────────────────────────────────────────────────

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
pub struct CarVisual;

#[derive(Component, Debug, Clone)]
pub struct VehicleData {
    pub lateral_velocity: f32,
    pub grip_state: GripState,
    pub surface_normal: Vec3,
    pub grounded: bool,
    pub forward_speed: f32,
}

impl Default for VehicleData {
    fn default() -> Self {
        Self {
            lateral_velocity: 0.0,
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

// ─── Resources ───────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct VehiclePhysicsConfig {
    // Suspension (Concept 3: Raycast Inset)
    pub suspension_stiffness: f32,
    pub suspension_damping: f32,
    pub suspension_rest_length: f32,
    pub raycast_inset: f32,
    pub raycast_max_distance: f32,

    // Surface Alignment (Concept 1)
    pub slerp_speed: f32,
    pub normal_smooth_speed: f32,
    pub custom_gravity: f32,
    pub downforce_base: f32,
    pub downforce_speed_factor: f32,

    // Tire Slip (Concept 2)
    pub slip_threshold: f32,
    pub lateral_stiffness: f32,
    pub kinetic_friction: f32,

    // Suspension force limits
    pub max_compression_ratio: f32,
    pub max_suspension_force: f32,

    // Drive
    pub engine_force: f32,
    pub brake_force: f32,
    pub steer_torque: f32,
    pub rolling_resistance: f32,
    pub drag_coefficient: f32,
    pub max_speed: f32,

    // Steering animation
    pub steer_smoothing: f32,
    pub max_steer_angle: f32,
    pub steer_speed_response: f32,

    // Mass (must match collider Mass component)
    pub mass: f32,
}

impl Default for VehiclePhysicsConfig {
    fn default() -> Self {
        Self {
            suspension_stiffness: 3500.0,
            suspension_damping: 400.0,
            suspension_rest_length: 0.45,
            raycast_inset: 0.4,
            raycast_max_distance: 5.0,
            slerp_speed: 10.0,
            normal_smooth_speed: 12.0,
            custom_gravity: 15.0,
            downforce_base: 5.0,
            downforce_speed_factor: 0.5,
            slip_threshold: 3.0,
            lateral_stiffness: 500.0,
            kinetic_friction: 1500.0,
            max_compression_ratio: 2.0,
            max_suspension_force: 8000.0,
            engine_force: 600.0,
            brake_force: 1500.0,
            steer_torque: 120.0,
            rolling_resistance: 8.0,
            drag_coefficient: 0.35,
            max_speed: 40.0,
            steer_smoothing: 10.0,
            max_steer_angle: 0.45,
            steer_speed_response: 25.0,
            mass: 15.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct WheelHitResult {
    pub distance: f32,
    pub normal: Vec3,
    pub compression: f32,
}

#[derive(Resource)]
pub struct SuspensionState {
    pub wheel_hits: [Option<WheelHitResult>; 4],
    pub average_normal: Vec3,
    pub smoothed_normal: Vec3,
    pub grounded_count: usize,
    pub ray_origins: [Vec3; 4],
}

impl Default for SuspensionState {
    fn default() -> Self {
        Self {
            wheel_hits: [None, None, None, None],
            average_normal: Vec3::Y,
            smoothed_normal: Vec3::Y,
            grounded_count: 0,
            ray_origins: [Vec3::ZERO; 4],
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
    pub roll: f32,
    pub braking: bool,
    pub boosting: bool,
}

#[derive(Resource, Default)]
pub struct CarColliderEntities(pub Vec<Entity>);

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

// ─── Constants ───────────────────────────────────────────────────────────────

const WHEEL_OFFSETS: [(f32, f32, f32); 4] = [
    (0.4, -0.8, 0.8),
    (-0.4, -0.8, 0.8),
    (0.4, -0.8, -0.8),
    (-0.4, -0.8, -0.8),
];

const FIXED_DT: f32 = 1.0 / 60.0;

// ─── System 1: Raycast Suspension (Concept 3: Raycast Inset) ─────────────────
//
// Casts rays from each wheel position, offset upward by the inset amount
// to prevent tunneling through steep loop entries.
// Compression = Rest_Length - (Hit_Distance - Inset_Offset)

fn raycast_suspension_system(
    spatial_query: SpatialQuery,
    children_query: Query<&Children>,
    config: Res<VehiclePhysicsConfig>,
    mut suspension_state: ResMut<SuspensionState>,
    mut car_colliders: ResMut<CarColliderEntities>,
    mut forces_query: Query<(Entity, Forces), With<PlayerCar>>,
) {
    let Ok((car_entity, forces)) = forces_query.single_mut() else {
        return;
    };

    if car_colliders.0.len() <= 1 {
        car_colliders.0.clear();
        car_colliders.0.push(car_entity);
        collect_descendants(&children_query, car_entity, &mut car_colliders.0);
    }

    let filter = SpatialQueryFilter::from_excluded_entities(car_colliders.0.iter().copied());

    let car_pos = forces.position().0;
    let car_rot = forces.rotation().0;
    let local_up = car_rot * Vec3::Y;
    let local_down = -local_up;

    let mut hits: [Option<WheelHitResult>; 4] = [None, None, None, None];
    let mut ray_origins = [Vec3::ZERO; 4];
    let mut normal_sum = Vec3::ZERO;
    let mut grounded_count = 0usize;

    let max_compression = config.suspension_rest_length * config.max_compression_ratio;

    for (i, &(lx, ly, lz)) in WHEEL_OFFSETS.iter().enumerate() {
        let wheel_local = Vec3::new(lx, ly, lz);
        let wheel_world = car_pos + car_rot * wheel_local;

        let ray_origin = wheel_world + local_up * config.raycast_inset;
        ray_origins[i] = ray_origin;

        let extended_max_dist = config.raycast_max_distance + config.raycast_inset;

        let mut best_eff_dist: Option<(f32, Vec3)> = None;

        for dir in [local_down, local_up] {
            let Ok(direction) = Dir3::new(dir) else { continue };
            if let Some(hit) = spatial_query.cast_ray(
                ray_origin,
                direction,
                extended_max_dist,
                true,
                &filter,
            ) {
                let effective_distance = hit.distance - config.raycast_inset;
                let raw_compression = config.suspension_rest_length - effective_distance;

                if raw_compression > 0.0 {
                    let is_better = best_eff_dist
                        .as_ref()
                        .map_or(true, |(prev_dist, _)| effective_distance < *prev_dist);
                    if is_better {
                        best_eff_dist = Some((effective_distance, hit.normal));
                    }
                }
            }
        }

        if let Some((effective_distance, normal)) = best_eff_dist {
            let raw_compression = config.suspension_rest_length - effective_distance;
            let compression = raw_compression.min(max_compression);
            hits[i] = Some(WheelHitResult {
                distance: effective_distance,
                normal,
                compression,
            });
            normal_sum += normal;
            grounded_count += 1;
        }
    }

    let raw_average = if grounded_count > 0 {
        normal_sum.normalize_or(Vec3::Y)
    } else {
        Vec3::Y
    };

    // Exponentially smooth the surface normal to prevent sudden direction changes
    let smooth_factor = 1.0 - (-config.normal_smooth_speed * FIXED_DT).exp();
    let smoothed = suspension_state
        .smoothed_normal
        .lerp(raw_average, smooth_factor)
        .normalize_or(Vec3::Y);

    *suspension_state = SuspensionState {
        wheel_hits: hits,
        average_normal: smoothed,
        smoothed_normal: smoothed,
        grounded_count,
        ray_origins,
    };
}

fn collect_descendants(
    children_query: &Query<&Children>,
    entity: Entity,
    out: &mut Vec<Entity>,
) {
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            out.push(child);
            collect_descendants(children_query, child, out);
        }
    }
}

// ─── System 2: Surface Alignment (Concept 1: Dynamic Surface Alignment & Artificial Gravity) ─
//
// Computes the average surface normal N_avg = (1/k) * Σ(n_i) for all
// grounded wheels, then SLERPs the vehicle's local Y-axis toward N_avg.
// Angular velocity is corrected to preserve only the yaw (steering) component.

fn surface_alignment_system(
    mut query: Query<
        (&mut AngularVelocity, &Rotation),
        With<PlayerCar>,
    >,
    susp: Res<SuspensionState>,
    config: Res<VehiclePhysicsConfig>,
) {
    let Ok((mut angular_vel, rotation)) = query.single_mut() else {
        return;
    };

    if susp.grounded_count == 0 {
        return;
    }

    let n_avg = susp.average_normal;

    let current_forward = rotation.0 * Vec3::Z;
    let mut z_local = current_forward - current_forward.dot(n_avg) * n_avg;
    if z_local.length_squared() < 1e-6 {
        let current_right = rotation.0 * Vec3::X;
        z_local = current_right.cross(n_avg);
    }
    z_local = z_local.normalize_or(current_forward);

    let y_local = n_avg;
    let x_local = y_local.cross(z_local).normalize_or(rotation.0 * Vec3::X);
    let z_local = x_local.cross(y_local).normalize_or(z_local);

    let target_rot = Quat::from_mat3(&Mat3::from_cols(x_local, y_local, z_local));

    // Compute angular velocity needed to SLERP toward target orientation
    let slerp_factor = 1.0 - (-config.slerp_speed * FIXED_DT).exp();
    let delta_rot = target_rot * rotation.0.inverse();

    // Extract rotation axis and angle, convert to angular velocity
    let (axis, angle) = delta_rot.to_axis_angle();
    let target_ang_vel = if angle.abs() > 1e-6 {
        axis * (angle * slerp_factor / FIXED_DT)
    } else {
        Vec3::ZERO
    };

    // Preserve only yaw (Y component in local space) from driver input,
    // blend alignment correction for roll and pitch
    let local_current = rotation.0.inverse() * angular_vel.0;
    let local_alignment = rotation.0.inverse() * target_ang_vel;

    angular_vel.0 = rotation.0 * Vec3::new(local_alignment.x, local_current.y, local_alignment.z);
}

// ─── System 3: Vehicle Forces (Concepts 1, 2, 3 + Drive) ───────────────────

fn apply_vehicle_forces(
    input: Res<CarInput>,
    config: Res<VehiclePhysicsConfig>,
    susp: Res<SuspensionState>,
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
    let up = car_rot * Vec3::Y;
    let forward_speed = velocity.dot(forward);

    let n_avg = susp.average_normal;
    let grounded = susp.grounded_count > 0;

    // ─── Concept 1: F_downforce = -N_avg * mass * custom_gravity_multiplier ─

    if grounded {
        let gravity_force = -n_avg * config.mass * config.custom_gravity;
        forces.apply_force(gravity_force);

        let speed_downforce = -n_avg * config.downforce_speed_factor * forward_speed.abs();
        forces.apply_force(speed_downforce);

        forces.apply_force(-n_avg * config.downforce_base * config.mass);
    } else {
        forces.apply_force(Vec3::Y * -config.custom_gravity * config.mass);
    }

    // ─── Concept 3: Suspension Spring Forces ─────────────────────────────────

    if grounded {
        for hit in susp.wheel_hits.iter().flatten() {
            if hit.compression > 0.0 {
                let spring_force = config.suspension_stiffness * hit.compression;

                let vel_along_normal = velocity.dot(hit.normal);
                let damping_force = -config.suspension_damping * vel_along_normal;

                let total_force = (spring_force + damping_force)
                    .clamp(0.0, config.max_suspension_force);
                forces.apply_force(hit.normal * total_force);
            }
        }
    }

    // ─── Concept 2: Two-State Tire Slip & Drift Friction ────────────────────
    // v_x = V_global · X_local (lateral velocity in vehicle's right direction)
    let lateral_vel = velocity.dot(right);
    let abs_lateral = lateral_vel.abs();

    if grounded {
        if abs_lateral < config.slip_threshold {
            // ── Static Grip State ──
            // Exact counter-impulse: completely negate lateral sliding
            // (-v_x * stiffness). We zero lateral velocity directly for
            // deterministic ML training behavior.
            let lateral_component = right * lateral_vel;
            *forces.linear_velocity_mut() -= lateral_component;
            vehicle_data.grip_state = GripState::Static;
        } else {
            // ── Kinetic Slip State (Drifting) ──
            // Break traction; clamp lateral counter-force to a constant
            // kinetic limit: F = -sign(v_x) * friction_kinetic
            let friction_dir = -right * lateral_vel.signum();
            forces.apply_force(friction_dir * config.kinetic_friction);
            vehicle_data.grip_state = GripState::Kinetic;
        }
    }

    // Store observation data for ML wrapper
    vehicle_data.lateral_velocity = lateral_vel;
    vehicle_data.surface_normal = n_avg;
    vehicle_data.grounded = grounded;
    vehicle_data.forward_speed = forward_speed;

    // ─── Drive Forces ──────────────────────────────────────────────────────

    let boost = if input.boosting { 1.5 } else { 1.0 };

    if grounded {
        let surface_forward = (forward - forward.dot(n_avg) * n_avg).normalize_or(forward);
        let engine = input.throttle * config.engine_force * boost;
        forces.apply_force(surface_forward * engine);
    } else {
        let engine = input.throttle * config.engine_force * boost * 0.3;
        forces.apply_force(forward * engine);
    }

    if input.braking {
        forces.apply_force(-velocity.normalize_or(Vec3::ZERO) * config.brake_force);
    }

    let steer_strength = (forward_speed.abs() / config.steer_speed_response)
        .min(1.0)
        .max(0.1);
    forces.apply_torque(up * input.steer * config.steer_torque * steer_strength);

    if input.roll != 0.0 && up.dot(Vec3::Y) <= 0.0 {
        forces.apply_torque(forward * input.roll * config.steer_torque * 0.5);
    }

    forces.apply_force(-velocity * config.rolling_resistance);
    forces.apply_force(-velocity * speed * config.drag_coefficient);

    if speed > config.max_speed {
        let excess = speed - config.max_speed;
        *forces.linear_velocity_mut() *= 1.0 - (excess / speed);
    }
}

// ─── Input & State Sync ────────────────────────────────────────────────────

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

    input.roll = 0.0;

    input.braking = keys.pressed(KeyCode::Space);
    input.boosting = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
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

    let dt: f32 = 1.0 / 60.0;
    let speed_delta = (forward_speed - car_state.prev_speed).abs() / dt.max(0.001);

    let is_kinetic = vehicle_data
        .iter()
        .next()
        .map_or(false, |d| d.grip_state == GripState::Kinetic);

    car_state.prev_speed = car_state.speed;
    car_state.speed = forward_speed;
    car_state.yaw = yaw;
    car_state.position = position.0;
    car_state.braking = input.braking;
    car_state.boosting = input.boosting;
    car_state.skidding = input.braking || is_kinetic || speed_delta > 25.0;
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
            commands.entity(car_entity).with_children(|parent| {
                for &(lx, _ly, lz) in WHEEL_OFFSETS.iter() {
                    parent.spawn((
                        Collider::cylinder(0.3, 0.25),
                        Transform::from_xyz(lx, -0.15, lz)
                            .with_rotation(Quat::from_rotation_z(PI / 2.0)),
                    ));
                }
            });
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