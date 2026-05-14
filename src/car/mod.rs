use bevy::prelude::*;

pub struct CarPlugin;

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, car_movement)
            .add_systems(Update, camera_follow);
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

fn car_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Car, With<PlayerCar>>,
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

        let accel = 40.0;
        let brake_force = 80.0;
        let friction = 3.0;
        let max_speed = 120.0;
        let steer_rate = 2.5;

        // Steering scales down at high speed for feel
        let steer = steer_input * steer_rate * (1.0 - (car.speed / max_speed).abs() * 0.5);

        if braking {
            car.speed -= car.speed.signum() * brake_force * dt;
            if car.speed.abs() < 1.0 {
                car.speed = 0.0;
            }
        } else {
            car.speed += (throttle * accel - car.speed * friction) * dt;
        }

        car.speed = car.speed.clamp(-max_speed * 0.3, max_speed);
        car.yaw += steer * dt * (car.speed / 30.0).clamp(-1.0, 1.0);
    }
}

fn camera_follow(
    car_query: Query<(&Car, &Transform), With<PlayerCar>>,
    mut cam_query: Query<&mut Transform, (With<CarCamera>, Without<Car>)>,
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