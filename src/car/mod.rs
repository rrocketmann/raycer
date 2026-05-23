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
            .init_resource::<CarColliderEntities>()
            .init_resource::<SuspensionState>()
            .add_systems(FixedPostUpdate, apply_car_forces.in_set(PhysicsSystems::Prepare))
            .add_systems(
                Update,
                (
                    capture_input,
                    sync_car_state,
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

#[derive(Resource, Default)]
pub struct SuspensionState {
    pub hits: [Option<f32>; 4],
    pub compressions: [f32; 4],
    pub forces: [f32; 4],
    pub ray_origins: [Vec3; 4],
    pub ray_hit_positions: [Option<Vec3>; 4],
    pub hit_dir_signs: [f32; 4], // +1 = +up, -1 = -up
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

fn collect_descendants(children_query: &Query<&Children>, entity: Entity, out: &mut Vec<Entity>) {
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            out.push(child);
            collect_descendants(children_query, child, out);
        }
    }
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
    pub engine_force: f32,
    pub brake_force: f32,
    pub steer_torque: f32,
    pub lateral_grip: f32,
    pub max_lateral_force: f32,
    pub rolling_resistance: f32,
    pub drag: f32,
    pub downforce: f32,
    #[allow(dead_code)]
    pub suspension_stiffness: f32,
    #[allow(dead_code)]
    pub suspension_damping: f32,
    #[allow(dead_code)]
    pub suspension_rest_length: f32,
}

impl Default for CarParams {
    fn default() -> Self {
        Self {
            engine_force: 500.0,
            brake_force: 1000.0,
            steer_torque: 800.0,
            lateral_grip: 300.0,
            max_lateral_force: 10000.0,
            rolling_resistance: 15.0,
            drag: 0.45,
            downforce: 3.0,
            suspension_stiffness: 800.0,
            suspension_damping: 30.0,
            suspension_rest_length: 1.2,
        }
    }
}

#[derive(Resource)]
pub struct WheelState {
    pub current_angle: f32,
    pub target_angle: f32,
}

impl Default for WheelState {
    fn default() -> Self {
        Self {
            current_angle: 0.0,
            target_angle: 0.0,
        }
    }
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

    input.roll = if keys.pressed(KeyCode::KeyQ) {
        -1.0
    } else if keys.pressed(KeyCode::KeyE) {
        1.0
    } else {
        0.0
    };

    input.braking = keys.pressed(KeyCode::Space);
    input.boosting = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
}

const WHEEL_OFFSETS: [(f32, f32, f32); 4] = [
    (0.4, -0.8, 0.8),
    (-0.4, -0.8, 0.8),
    (0.4, -0.8, -0.8),
    (-0.4, -0.8, -0.8),
];

fn apply_car_forces(
    input: Res<CarInput>,
    params: Res<CarParams>,
    spatial_query: SpatialQuery,
    children_query: Query<&Children>,
    mut query: Query<(Entity, Forces), With<PlayerCar>>,
    mut car_colliders: ResMut<CarColliderEntities>,
    mut suspension_state: ResMut<SuspensionState>,
) {
    let Ok((car_entity, mut forces)) = query.single_mut() else {
        return;
    };

    if car_colliders.0.len() <= 1 {
        car_colliders.0.clear();
        car_colliders.0.push(car_entity);
        collect_descendants(&children_query, car_entity, &mut car_colliders.0);
    }

    let forward = forces.rotation().0 * Vec3::Z;
    let _right = forces.rotation().0 * Vec3::X;
    let up = forces.rotation().0 * Vec3::Y;
    let velocity = forces.linear_velocity();
    let speed = velocity.length();
    let forward_speed = velocity.dot(forward);

    let boost = if input.boosting { 1.4 } else { 1.0 };

    let engine = input.throttle * params.engine_force * boost;
    forces.apply_force(forward * engine);

    if input.braking {
        forces.apply_force(-velocity.normalize_or_zero() * params.brake_force);
    }

    let steer_strength = (forward_speed.abs() / 30.0).min(1.0);
    forces.apply_torque(Vec3::Y * input.steer * params.steer_torque * steer_strength);

    // Lateral grip handled by contact friction — no artificial grip force

    forces.apply_force(-velocity * params.rolling_resistance);
    forces.apply_force(-velocity * speed * params.drag);

    let world_up = Vec3::Y;
    if input.roll != 0.0 && up.dot(world_up) > 0.0 {
        forces.apply_torque(forward * input.roll * params.steer_torque * 0.5);
    }

    if up.dot(world_up) > 0.0 {
        forces.apply_force(-up * params.downforce * forward_speed.abs().max(5.0));
    }

    // Box collider keeps the car level — no anti-roll torque needed

    let filter = SpatialQueryFilter::from_excluded_entities(car_colliders.0.iter().copied());
    let mut susp = SuspensionState::default();
    for (i, &(lx, ly, lz)) in WHEEL_OFFSETS.iter().enumerate() {
        let wheel_world = forces.position().0 + forces.rotation().0 * Vec3::new(lx, ly, lz);
        susp.ray_origins[i] = wheel_world;
        let mut best: Option<(f32, f32, Vec3, Vec3)> = None;
        for ray_dir in [-up, up] {
            if let Some(hit) = spatial_query.cast_ray(
                wheel_world,
                Dir3::new(ray_dir).unwrap_or(Dir3::NEG_Y),
                10.0,
                false,
                &filter,
            ) {
                let compression = params.suspension_rest_length - hit.distance;
                if compression > 0.0 && best.map_or(true, |(best_comp, _, _, _)| compression > best_comp) {
                    best = Some((compression, hit.distance, wheel_world + ray_dir * hit.distance, ray_dir));
                }
            }
        }
        if let Some((compression, dist, hit_pos, ray_dir)) = best {
            susp.hits[i] = Some(dist);
            susp.compressions[i] = compression;
            susp.ray_hit_positions[i] = Some(hit_pos);
            susp.hit_dir_signs[i] = ray_dir.dot(up).signum();
            // No suspension force — raycasts are for ground-contact detection only
        }
    }
    *suspension_state = susp;
}

fn sync_car_state(
    input: Res<CarInput>,
    mut car_query: Query<(&LinearVelocity, &AngularVelocity, &Rotation, &Position, &mut Car), With<PlayerCar>>,
    mut car_state: ResMut<CarState>,
) {
    let Ok((lin_vel, _ang_vel, rotation, position, mut car)) = car_query.single_mut() else {
        return;
    };

    let forward = rotation.0 * Vec3::Z;
    let forward_speed = lin_vel.0.dot(forward);
    let yaw = forward.x.atan2(forward.z);

    car.speed = forward_speed;
    car.yaw = yaw;

    let dt: f32 = 1.0 / 60.0;
    let speed_delta = (forward_speed - car_state.prev_speed).abs() / dt.max(0.001);
    car_state.skidding = input.braking || speed_delta > 25.0;

    car_state.prev_speed = car_state.speed;
    car_state.speed = forward_speed;
    car_state.yaw = yaw;
    car_state.position = position.0;
    car_state.braking = input.braking;
    car_state.boosting = input.boosting;
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
                            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
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
    _susp: Res<SuspensionState>,
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
    susp: Res<SuspensionState>,
    car_query: Query<(&Position, &Transform, &Rotation, &LinearVelocity, &AngularVelocity, &Car), With<PlayerCar>>,
) {
    *frame += 1;
    if *frame % 30 != 0 {
        return;
    }
    use std::f32::consts::PI;
    let Ok((pos, tf, rot, vel, ang_vel, car)) = car_query.single() else {
        return;
    };
    let (roll, yaw, pitch) = rot.0.to_euler(EulerRot::ZYX);
    info!(
        "\n─── frame={} ───\n\
         input: t={:.2} s={:.2} brake={} boost={}\n\
         pos: ({:.2}, {:.2}, {:.2})  tf_y={:.2}  yaw={:.1}° pitch={:.1}° roll={:.1}°\n\
         vel: ({:.2}, {:.2}, {:.2})  speed={:.1}  fwd={:.1}\n\
         ang_vel: ({:.2}, {:.2}, {:.2})\n\
         susp: FL(dist={:.3}, comp={:.3}, force={:.0}) | FR(d={:.3}, c={:.3}, f={:.0}) | RL(d={:.3}, c={:.3}, f={:.0}) | RR(d={:.3}, c={:.3}, f={:.0})\n\
         skid={}",
        *frame,
        input.throttle, input.steer, input.braking, input.boosting,
        pos.0.x, pos.0.y, pos.0.z, tf.translation.y, yaw * 180.0 / PI, pitch * 180.0 / PI, roll * 180.0 / PI,
        vel.0.x, vel.0.y, vel.0.z, vel.0.length(), car.speed,
        ang_vel.0.x, ang_vel.0.y, ang_vel.0.z,
        susp.hits[0].unwrap_or(-1.0), susp.compressions[0], susp.forces[0],
        susp.hits[1].unwrap_or(-1.0), susp.compressions[1], susp.forces[1],
        susp.hits[2].unwrap_or(-1.0), susp.compressions[2], susp.forces[2],
        susp.hits[3].unwrap_or(-1.0), susp.compressions[3], susp.forces[3],
        car_state.skidding,
    );
}
