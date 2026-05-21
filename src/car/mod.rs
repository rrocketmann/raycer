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
            .init_resource::<SkidOffsets>()
            .init_resource::<GroundInfo>()
            .add_systems(Startup, setup_skid_assets)
            .add_systems(
                FixedPostUpdate,
                apply_car_forces.in_set(PhysicsSystems::Prepare),
            )
            .add_systems(
                FixedPostUpdate,
                sync_car_state.in_set(PhysicsSystems::Last),
            )
            .add_systems(
                Update,
                (
                    capture_input,
                    ground_raycast,
                    camera_follow,
                    label_wheels,
                    animate_wheels,
                    record_telemetry,
                    spawn_skid_marks,
                    fade_skid_marks,
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

pub const GROUND_RAY_DISTANCE: f32 = 100.0;

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
    pub engine_force: f32,
    pub max_speed: f32,
    pub brake_force: f32,
    pub steer_torque: f32,
    pub lateral_grip: f32,
    pub max_lateral_force: f32,
    pub rolling_resistance: f32,
    pub drag: f32,
    pub downforce: f32,
}

impl Default for CarParams {
    fn default() -> Self {
        Self {
            engine_force: 4200.0,
            max_speed: 80.0,
            brake_force: 5200.0,
            steer_torque: 2200.0,
            lateral_grip: 120.0,
            max_lateral_force: 4200.0,
            rolling_resistance: 12.0,
            drag: 0.35,
            downforce: 2.5,
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

fn apply_car_forces(
    input: Res<CarInput>,
    params: Res<CarParams>,
    mut query: Query<(Forces, &Rotation, &LinearVelocity), With<PlayerCar>>,
) {
    let Ok((mut forces, rotation, linear_velocity)) = query.single_mut() else {
        return;
    };

    let forward = rotation.0 * Vec3::NEG_Z;
    let right = rotation.0 * Vec3::X;
    let velocity = linear_velocity.0;
    let speed = velocity.length();
    let forward_speed = velocity.dot(forward);
    let lateral_speed = velocity.dot(right);

    let boost_multiplier = if input.boosting { 1.4 } else { 1.0 };
    let engine_force = input.throttle * params.engine_force * boost_multiplier;
    forces.apply_force(forward * engine_force);

    if input.braking && speed > 0.5 {
        let brake_force = -velocity.normalize_or_zero() * params.brake_force;
        forces.apply_force(brake_force);
    }

    let lateral_force = (-lateral_speed * params.lateral_grip)
        .clamp(-params.max_lateral_force, params.max_lateral_force);
    forces.apply_force(right * lateral_force);

    let rolling_resistance = -velocity * params.rolling_resistance;
    let drag = -velocity * speed * params.drag;
    forces.apply_force(rolling_resistance + drag);

    let steer_strength = (forward_speed.abs() / params.max_speed).clamp(0.2, 1.0);
    let steer_torque = input.steer * params.steer_torque * steer_strength;
    forces.apply_torque(Vec3::Y * steer_torque);

    let downforce = params.downforce * speed * speed;
    if downforce > 0.0 {
        forces.apply_force(Vec3::NEG_Y * downforce);
    }
}

fn sync_car_state(
    time: Res<Time>,
    input: Res<CarInput>,
    mut car_state: ResMut<CarState>,
    mut car_query: Query<(
        &LinearVelocity,
        &AngularVelocity,
        &Rotation,
        &Position,
        &mut Car,
    ), With<PlayerCar>>,
) {
    let Ok((linear_velocity, angular_velocity, rotation, position, mut car)) = car_query.single_mut()
    else {
        return;
    };

    let forward = rotation.0 * Vec3::NEG_Z;
    let right = rotation.0 * Vec3::X;
    let velocity = linear_velocity.0;
    let forward_speed = velocity.dot(forward);
    let lateral_speed = velocity.dot(right);
    let yaw_rate = angular_velocity.0.y;
    let yaw = (-forward.x).atan2(-forward.z);
    let dt = time.delta_secs().max(0.001);

    car.speed = forward_speed;
    car.yaw = yaw;

    let speed_delta = (forward_speed - car_state.prev_speed).abs() / dt;
    let drift_turn = lateral_speed.abs() > 3.5 && forward_speed.abs() > 6.0 && yaw_rate.abs() > 0.3;

    car_state.skidding = (input.braking && forward_speed.abs() > 8.0)
        || (input.throttle == 0.0 && forward_speed.abs() > 35.0)
        || (speed_delta > 18.0 && forward_speed.abs() > 8.0)
        || drift_turn;
    car_state.prev_speed = forward_speed;
    car_state.speed = forward_speed;
    car_state.yaw = yaw;
    car_state.braking = input.braking;
    car_state.boosting = input.boosting;
    car_state.position = position.0;
}

fn camera_follow(
    car_query: Query<(&Car, &Position), With<PlayerCar>>,
    mut cam_query: Query<&mut Transform, (With<CarCamera>, Without<PlayerCar>)>,
) {
    let Some((car, car_pos)) = car_query.iter().next() else {
        return;
    };

    let car_pos = car_pos.0;
    let forward = Vec3::new(-car.yaw.sin(), 0.0, -car.yaw.cos());
    let behind = -forward * 10.0;
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

    let speed_factor = (car.speed / params.max_speed).abs().clamp(0.0, 1.0);
    let effective_steer = steer_input * (1.0 - speed_factor * 0.4);
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
    car_query: Query<(&Car, &AngularVelocity), With<PlayerCar>>,
) {
    let Ok((car, angular_velocity)) = car_query.single() else {
        return;
    };
    let yaw_rate = angular_velocity.0.y;
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
    car_state: Res<CarState>,
    params: Res<CarParams>,
    ground_info: Res<GroundInfo>,
    skid_assets: Res<SkidMarkAssets>,
    skid_offsets: Res<SkidOffsets>,
    skid_count: Query<(), With<SkidMark>>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut last_spawn: Local<Option<Vec3>>,
    mut last_position: Local<Option<Vec3>>,
    mut distance_accum: Local<f32>,
) {
    if !car_state.skidding || car_state.speed.abs() < 10.0 {
        *last_spawn = None;
        *last_position = None;
        *distance_accum = 0.0;
        return;
    }

    let mut remaining = 600 - skid_count.iter().count();
    if remaining < 2 {
        return;
    }

    let speed_ratio = (car_state.speed.abs() / params.max_speed).clamp(0.0, 1.0);
    let spacing = (0.18 - 0.12 * speed_ratio).max(0.06);
    let last_pos = match *last_position {
        Some(pos) => pos,
        None => {
            *last_spawn = Some(car_state.position);
            *last_position = Some(car_state.position);
            *distance_accum = 0.0;
            return;
        }
    };
    let delta = car_state.position - last_pos;
    let dist = delta.length();
    *last_position = Some(car_state.position);
    if dist <= f32::EPSILON {
        return;
    }
    let dir = delta / dist;
    *distance_accum += dist;
    let mut spawn_pos = last_spawn.unwrap_or(last_pos);

    let speed_ratio = ((car_state.speed.abs() - 15.0) / 80.0).min(1.0);
    let base_alpha = 0.4 + 0.4 * speed_ratio;

    let forward = Vec3::new(-car_state.yaw.sin(), 0.0, -car_state.yaw.cos());
    let right = Vec3::new(-car_state.yaw.cos(), 0.0, car_state.yaw.sin());
    let yaw_quat = Quat::from_rotation_y(car_state.yaw);
    let normal = ground_info.normal;
    let surface_align = Quat::from_rotation_arc(Vec3::Y, normal);
    let rotation = surface_align * yaw_quat * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    let skid_y = ground_info.y + 0.02;

    while *distance_accum >= spacing && remaining >= 2 {
        spawn_pos += dir * spacing;
        *distance_accum -= spacing;
        *last_spawn = Some(spawn_pos);
        remaining -= 2;

        for lateral in [skid_offsets.left, skid_offsets.right] {
            let pos = spawn_pos - forward * 1.0 + right * lateral;
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
