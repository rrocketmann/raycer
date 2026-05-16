use bevy::prelude::*;

use crate::track::MinimapCamera;

pub struct CarPlugin;

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CarParams>()
            .init_resource::<Telemetry>()
            .init_resource::<WheelState>()
            .add_systems(FixedUpdate, car_movement)
            .add_systems(Update, (camera_follow, update_car_visuals, label_wheels, animate_wheels, record_telemetry, update_minimap_camera));
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

pub const MAP_HALF_SIZE: f32 = 60.0;

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

fn car_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    params: Res<CarParams>,
    mut query: Query<&mut Car, With<PlayerCar>>,
    mut car_transform: Query<&mut Transform, (With<PlayerCar>, With<CarVisual>)>,
) {
    let dt = time.delta_secs();

    for mut car in query.iter_mut() {
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

        let steer = steer_input * params.steer_rate * (1.0 - (car.speed / params.max_speed).abs() * 0.5);

        if braking {
            car.speed -= car.speed.signum() * params.brake_force * dt;
            if car.speed.abs() < 1.0 {
                car.speed = 0.0;
            }
        } else {
            car.speed += (throttle * params.acceleration - car.speed * params.friction) * dt;
        }

        let steer_friction = steer.abs() * car.speed.abs() * 0.02;
        car.speed -= car.speed.signum() * steer_friction * dt;

        car.speed = car.speed.clamp(-params.max_speed * 0.3, params.max_speed);
        car.yaw += steer * dt * (car.speed / 30.0).clamp(-1.0, 1.0);
    }

    if let Ok(car) = query.single() {
        for mut transform in car_transform.iter_mut() {
            let forward = Vec3::new(car.yaw.sin(), 0.0, car.yaw.cos());
            transform.translation += forward * car.speed * dt;
            let dist = (transform.translation.x * transform.translation.x + transform.translation.z * transform.translation.z).sqrt();
            if dist > MAP_HALF_SIZE {
                let scale = MAP_HALF_SIZE / dist;
                transform.translation.x *= scale;
                transform.translation.z *= scale;
            }
        }
    }
}

fn camera_follow(
    car_query: Query<(&Car, &Transform), With<PlayerCar>>,
    mut cam_query: Query<&mut Transform, (With<CarCamera>, Without<MinimapCamera>, Without<PlayerCar>)>,
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

fn update_minimap_camera(
    car_query: Query<(&Car, &Transform), With<PlayerCar>>,
    mut minimap_query: Query<&mut Transform, (With<MinimapCamera>, Without<CarCamera>, Without<PlayerCar>)>,
) {
    let Some((car, car_transform)) = car_query.iter().next() else { return };
    for mut transform in minimap_query.iter_mut() {
        let pos = Vec3::new(car_transform.translation.x, 120.0, car_transform.translation.z);
        let target = Vec3::new(car_transform.translation.x, 0.0, car_transform.translation.z);
        let up = Vec3::new(car.yaw.sin(), 0.0, car.yaw.cos());
        *transform = Transform::from_translation(pos).looking_at(target, up);
    }
}