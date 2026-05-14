use bevy::prelude::*;

pub struct CarPlugin;

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (car_movement, car_camera_follow));
    }
}

#[derive(Component)]
pub struct Car {
    pub speed: f32,
    pub max_speed: f32,
    pub acceleration: f32,
    pub braking: f32,
    pub steer_angle: f32,
    pub max_steer: f32,
    pub steer_speed: f32,
}

impl Default for Car {
    fn default() -> Self {
        Self {
            speed: 0.0,
            max_speed: 60.0,
            acceleration: 30.0,
            braking: 50.0,
            steer_angle: 0.0,
            max_steer: 0.6,
            steer_speed: 3.0,
        }
    }
}

#[derive(Component)]
pub struct PlayerCar;

#[derive(Default)]
pub struct CarInput {
    pub throttle: f32,
    pub steer: f32,
    pub brake: f32,
}

pub fn car_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut cars: Query<(Entity, &mut Car)>,
    mut transforms: Query<&mut Transform>,
) {
    let input = CarInput {
        throttle: if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
            1.0
        } else if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
            -1.0
        } else {
            0.0
        },
        steer: if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
            -1.0
        } else if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
            1.0
        } else {
            0.0
        },
        brake: if keys.pressed(KeyCode::Space) { 1.0 } else { 0.0 },
    };

    for (entity, mut car) in cars.iter_mut() {
        let dt = time.delta_secs();

        // Throttle/brake
        if input.throttle > 0.0 {
            car.speed += car.acceleration * input.throttle * dt;
        } else if input.throttle < 0.0 {
            car.speed -= car.acceleration * 0.5 * input.throttle.abs() * dt;
        }

        // Braking
        if input.brake > 0.0 {
            car.speed -= car.braking * input.brake * dt;
        }

        // Friction
        car.speed *= 1.0 - 2.0 * dt;

        // Clamp speed
        car.speed = car.speed.clamp(-car.max_speed * 0.3, car.max_speed);

        // Steering
        let target_steer = input.steer * car.max_steer;
        car.steer_angle += (target_steer - car.steer_angle) * car.steer_speed * dt;

        // Apply movement
        if let Ok(mut transform) = transforms.get_mut(entity) {
            let yaw = transform.rotation.to_euler(EulerRot::YXZ).1;
            let new_yaw = yaw + car.steer_angle * car.speed * dt * 0.05;
            transform.rotation = Quat::from_rotation_y(new_yaw);
            transform.translation.x += car.speed * dt * yaw.sin();
            transform.translation.z += car.speed * dt * yaw.cos();
        }
    }
}

pub fn car_camera_follow(
    cars: Query<&Transform, (With<Car>, With<PlayerCar>)>,
    mut cameras: Query<&mut Transform, (With<Camera>, Without<Car>)>,
) {
    if let Some(car_transform) = cars.iter().next() {
        for mut camera in cameras.iter_mut() {
            let offset = Vec3::new(0.0, 5.0, -10.0);
            let target = car_transform.translation + offset;
            camera.translation = camera.translation.lerp(target, 0.1);
            camera.look_at(car_transform.translation, Vec3::Y);
        }
    }
}
