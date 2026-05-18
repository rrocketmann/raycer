use bevy::prelude::*;

pub struct CarPlugin;

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CarParams>()
            .init_resource::<Telemetry>()
            .init_resource::<WheelState>()
            .init_resource::<CarState>()
            .init_resource::<SkidOffsets>()
            .add_systems(Startup, setup_skid_assets)
            .add_systems(FixedUpdate, car_movement)
            .add_systems(Update, (camera_follow, update_car_visuals, label_wheels, animate_wheels, record_telemetry, spawn_skid_marks, fade_skid_marks));
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
pub struct CarVisual;

#[derive(Component)]
pub struct WheelFrontLeft;

#[derive(Component)]
pub struct WheelFrontRight;

#[derive(Component)]
pub struct WheelRearLeft;

#[derive(Component)]
pub struct WheelRearRight;

#[derive(Resource)]
pub struct CarParams {
    pub max_speed: f32,
    pub acceleration: f32,
    pub brake_force: f32,
    pub friction: f32,
    pub steer_rate: f32,
}

impl Default for CarParams {
    fn default() -> Self {
        Self {
            max_speed: 240.0,
            acceleration: 80.0,
            brake_force: 160.0,
            friction: 3.0,
            steer_rate: 2.5,
        }
    }
}

pub const ARENA_RADIUS: f32 = 60.0;
pub const CAR_COLLISION_RADIUS: f32 = 1.3;

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
    fn push(&mut self, speed: f32, steer: f32, yaw_rate: f32) {
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

fn car_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    params: Res<CarParams>,
    mut query: Query<&mut Car, With<PlayerCar>>,
    mut car_transform: Query<&mut Transform, (With<PlayerCar>, With<CarVisual>)>,
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

    for mut car in query.iter_mut() {
        let boost_multiplier = if boosting { 1.5 } else { 1.0 };
        let steer_penalty = if boosting { 0.5 } else { 1.0 };
        let max_speed_boosted = params.max_speed * boost_multiplier;
        let steer = steer_input * params.steer_rate * steer_penalty * (1.0 - (car.speed / max_speed_boosted).abs() * 0.5);

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

        car.speed = car.speed.clamp(-params.max_speed * 0.3, max_speed_boosted);
        car.yaw += steer * dt * (car.speed / 30.0).clamp(-1.0, 1.0);
    }

    if let Ok(mut car) = query.single_mut() {
        let mut pos = Vec3::ZERO;
        for mut transform in car_transform.iter_mut() {
            let forward = Vec3::new(car.yaw.sin(), 0.0, car.yaw.cos());
            transform.translation += forward * car.speed * dt;
            let car_center = transform.translation - forward * 1.2;
            let effective_radius = ARENA_RADIUS - CAR_COLLISION_RADIUS;
            let dist = (car_center.x * car_center.x + car_center.z * car_center.z).sqrt();
            if dist > effective_radius {
                let scale = effective_radius / dist;
                let offset = transform.translation - car_center;
                transform.translation = car_center * scale + offset;
                let wall_normal = Vec3::new(car_center.x, 0.0, car_center.z).normalize();
                let velocity = forward * car.speed;
                let vel_along_wall = velocity - wall_normal * velocity.dot(wall_normal);
                car.speed = vel_along_wall.dot(forward).signum() * vel_along_wall.length();
                car.speed *= 0.8;
            }
            pos = transform.translation;
        }
        let decel_rate = (car.speed / params.max_speed).abs() * params.friction;
        let speed_delta = (car.speed - car_state.prev_speed).abs() / dt.max(0.001);
        car_state.skidding = (braking && car.speed.abs() > 10.0) || (throttle == 0.0 && car.speed.abs() > 40.0 && decel_rate > 1.5) || (speed_delta > 25.0 && car.speed.abs() > 10.0);
        car_state.prev_speed = car.speed;
        car_state.speed = car.speed;
        car_state.yaw = car.yaw;
        car_state.braking = braking;
        car_state.boosting = boosting;
        car_state.position = pos;
    }
}

fn camera_follow(
    car_query: Query<(&Car, &Transform), With<PlayerCar>>,
    mut cam_query: Query<&mut Transform, (With<CarCamera>, Without<PlayerCar>)>,
) {
    let Some((car, car_transform)) = car_query.iter().next() else {
        return;
    };

    let car_pos = car_transform.translation;
    let behind = Vec3::new(car.yaw.sin(), 0.0, car.yaw.cos()) * -8.0;
    let up = Vec3::new(0.0, 5.0, 0.0);
    let target = car_pos + behind + up;

    for mut cam in cam_query.iter_mut() {
        cam.translation = cam.translation.lerp(target, 0.05);
        cam.look_at(car_pos + Vec3::new(0.0, 1.0, 0.0), Vec3::Y);
    }
}

fn update_car_visuals(
    mut car_query: Query<&mut Transform, (With<CarVisual>, With<PlayerCar>)>,
    car_data: Query<&Car, With<PlayerCar>>,
) {
    let Some(car) = car_data.iter().next() else {
        return;
    };
    for mut transform in car_query.iter_mut() {
        transform.rotation = Quat::from_rotation_y(car.yaw);
    }
}

#[derive(Component)]
pub struct WheelsLabeled;

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
    telemetry.push(car.speed, wheel_state.current_angle, yaw_rate);
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
    let rotation = Quat::from_rotation_y(car_state.yaw) * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);

    for lateral in [skid_offsets.left, skid_offsets.right] {
        let pos = car_state.position - forward * 1.0 + right * lateral;
        let pos = Vec3::new(pos.x, 0.02, pos.z);

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