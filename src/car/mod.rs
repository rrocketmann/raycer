use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::input::mouse::{MouseWheel, MouseMotion};
use bevy::ecs::message::MessageReader;
use rand::Rng;
use crate::GameState;
use crate::MaxHealthPoints;

pub const SKY_BOUNDARY: f32 = 50.0;

#[derive(Component)]
pub struct Health(pub u8);

#[derive(Component)]
pub struct HealthSegment(pub u8);

pub struct CarDef {
    pub name: &'static str,
    pub path: &'static str,
    pub collider: Vec3,
}

pub const CAR_DEFS: &[CarDef] = &[
    CarDef { name: "Race",           path: "models/race.glb",              collider: Vec3::new(1.20, 0.63, 2.56) },
    CarDef { name: "Race Future",    path: "models/race-future.glb",       collider: Vec3::new(1.20, 0.63, 2.66) },
    CarDef { name: "Hatchback",     path: "models/hatchback-sports.glb",  collider: Vec3::new(1.30, 0.95, 2.85) },
    CarDef { name: "Sedan",          path: "models/sedan.glb",             collider: Vec3::new(1.50, 1.15, 2.55) },
    CarDef { name: "Sedan Sport",    path: "models/sedan-sports.glb",      collider: Vec3::new(1.30, 0.95, 2.55) },
    CarDef { name: "SUV",            path: "models/suv.glb",               collider: Vec3::new(1.50, 1.10, 2.55) },
    CarDef { name: "SUV Luxury",     path: "models/suv-luxury.glb",        collider: Vec3::new(1.50, 1.18, 2.85) },
    CarDef { name: "Taxi",           path: "models/taxi.glb",              collider: Vec3::new(1.50, 1.35, 2.75) },
    CarDef { name: "Police",         path: "models/police.glb",            collider: Vec3::new(1.50, 1.10, 2.90) },
    CarDef { name: "Ambulance",      path: "models/ambulance.glb",          collider: Vec3::new(1.50, 1.60, 3.25) },
    CarDef { name: "Delivery",       path: "models/delivery.glb",           collider: Vec3::new(1.50, 1.50, 3.25) },
    CarDef { name: "Delivery Flat",  path: "models/delivery-flat.glb",      collider: Vec3::new(1.50, 1.20, 3.25) },
    CarDef { name: "Van",            path: "models/van.glb",               collider: Vec3::new(1.50, 1.20, 2.75) },
    CarDef { name: "Truck",          path: "models/truck.glb",             collider: Vec3::new(1.50, 1.15, 2.95) },
    CarDef { name: "Truck Flat",     path: "models/truck-flat.glb",        collider: Vec3::new(1.50, 1.15, 2.75) },
    CarDef { name: "Firetruck",      path: "models/firetruck.glb",          collider: Vec3::new(1.50, 1.50, 3.25) },
    CarDef { name: "Garbage",        path: "models/garbage-truck.glb",     collider: Vec3::new(1.50, 1.48, 3.45) },
    CarDef { name: "Tractor",        path: "models/tractor.glb",           collider: Vec3::new(1.34, 1.41, 1.98) },
    CarDef { name: "Tractor Police", path: "models/tractor-police.glb",    collider: Vec3::new(1.34, 1.51, 1.98) },
    CarDef { name: "Tractor Shovel", path: "models/tractor-shovel.glb",    collider: Vec3::new(1.46, 1.34, 2.00) },
];

pub fn mount_y(collider_y: f32) -> f32 {
    collider_y * 1.5 + 0.1
}

#[derive(Resource, Default)]
pub struct CarSelection {
    pub index: usize,
    pub pending_change: bool,
}

pub struct CarPlugin;

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehiclePhysicsConfig>()
            .init_resource::<WheelState>()
            .init_resource::<CarInput>()
            .init_resource::<CarState>()
            .init_resource::<Telemetry>()
            .init_resource::<GroundState>()
            .init_resource::<CarSelection>()
            .init_resource::<CameraState>()
            .add_systems(
                FixedPostUpdate,
                (ground_detection_system, apply_vehicle_forces, smooth_angular_velocity).chain().in_set(PhysicsSystems::Prepare).run_if(in_state(GameState::Playing)),
            )
            .add_systems(Update, capture_input.run_if(in_state(GameState::Playing)))
            .add_systems(Update, camera_input.run_if(in_state(GameState::Playing)))
            .add_systems(Update, (sync_car_state, clamp_speed, camera_follow).run_if(in_state(GameState::Playing)))
            .add_systems(
                Update,
                (
                    label_wheels,
                    animate_wheels,
                    record_telemetry,
                    respawn_car,
                    respawn_hit_cars,
                    switch_car_model,
                ).run_if(in_state(GameState::Playing)),
            )
            .add_systems(Update, switch_car_model_pregame.run_if(in_state(GameState::PreGame)))
            .add_systems(Update, update_health_indicators.run_if(in_state(GameState::Playing)))
            .add_systems(Update, (update_explosions, move_explosion_particles));
    }
}

pub fn spawn_health_indicators(
    car_entity: Entity,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    collider_y: f32,
    hp: u8,
) {
    let square_size = 0.25;
    let gap = 0.35;
    let count = hp as usize;
    let total_width = (count as f32 - 1.0) * gap;
    let start_x = -total_width * 0.5;
    let y_offset = collider_y + 2.5;
    let mesh = meshes.add(Cuboid::new(square_size, square_size, square_size * 2.0));
    let material = materials.add(Color::srgb(0.45, 0.45, 0.45));

    for i in 0..count {
        let x = start_x + i as f32 * gap;
        commands.entity(car_entity).with_children(|parent| {
            parent.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(x, y_offset, 0.0),
                HealthSegment(i as u8),
            ));
        });
    }
}

fn update_health_indicators(
    mut commands: Commands,
    health_query: Query<(Entity, &Health)>,
    segment_query: Query<&HealthSegment>,
    children_query: Query<&Children>,
    max_hp: Res<MaxHealthPoints>,
) {
    for (car_entity, health) in health_query.iter() {
        let hp = health.0;
        if hp >= max_hp.0 { continue; }
        for child in children_query.iter_descendants(car_entity) {
            if let Ok(segment) = segment_query.get(child) {
                if segment.0 >= hp {
                    commands.entity(child).despawn();
                }
            }
        }
    }
}

#[derive(Component)]
pub struct ExplosionTimer(pub Timer);

#[derive(Component)]
pub struct ExplosionParticle {
    pub velocity: Vec3,
    pub lifetime: Timer,
}

pub fn spawn_impact_effect(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
) {
    let mut rng = rand::rng();
    for _ in 0..5 {
        let dir = Vec3::new(
            rng.random_range(-1.0..1.0),
            rng.random_range(0.0..1.0),
            rng.random_range(-1.0..1.0),
        ).normalize_or(Vec3::Y);
        let speed = rng.random_range(5.0..20.0);
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.2).mesh().ico(1).unwrap())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Srgba::hex("ff8800").unwrap().into(),
                emissive: LinearRgba::new(4.0, 2.0, 0.0, 1.0),
                ..default()
            })),
            Transform::from_translation(position),
            ExplosionParticle {
                velocity: dir * speed,
                lifetime: Timer::from_seconds(rng.random_range(0.2..0.5), TimerMode::Once),
            },
        ));
    }
}

fn move_explosion_particles(
    time: Res<Time>,
    mut commands: Commands,
    mut particles: Query<(Entity, &mut Transform, &mut ExplosionParticle)>,
) {
    for (entity, mut transform, mut particle) in particles.iter_mut() {
        particle.lifetime.tick(time.delta());
        if particle.lifetime.just_finished() {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation += particle.velocity * time.delta_secs();
    }
}

fn update_explosions(
    time: Res<Time>,
    mut commands: Commands,
    mut explosion_query: Query<(Entity, &GlobalTransform, &mut ExplosionTimer)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, transform, mut timer) in explosion_query.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            commands.entity(entity).despawn();
            continue;
        }
        let mut rng = rand::rng();
        for _ in 0..3 {
            let dir = Vec3::new(
                rng.random_range(-1.0..1.0),
                rng.random_range(0.0..1.0),
                rng.random_range(-1.0..1.0),
            ).normalize_or(Vec3::Y);
            let speed = rng.random_range(15.0..40.0);
            commands.spawn((
                Mesh3d(meshes.add(Sphere::new(0.25).mesh().ico(1).unwrap())),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Srgba::hex("ff6600").unwrap().into(),
                    emissive: LinearRgba::new(6.0, 2.0, 0.0, 1.0),
                    ..default()
                })),
                Transform::from_translation(transform.translation()),
                ExplosionParticle {
                    velocity: dir * speed,
                    lifetime: Timer::from_seconds(rng.random_range(0.3..0.8), TimerMode::Once),
                },
            ));
        }
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
pub struct AiCar;

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
    pub rolling_resistance: f32,
    pub drag_coefficient: f32,
    pub max_speed: f32,
    pub boost_max_speed: f32,
    pub steer_smoothing: f32,
    pub max_steer_angle: f32,
    pub steer_speed_response: f32,
}

impl Default for VehiclePhysicsConfig {
    fn default() -> Self {
        Self {
            downforce: 30.0,
            downforce_speed: 0.02,
            lateral_stiffness: 150.0,
            slip_threshold: 4.0,
            kinetic_friction: 3000.0,
            engine_force: 2400.0,
            brake_force: 6000.0,
            steer_torque: 600.0,
            rolling_resistance: 3.0,
            drag_coefficient: 0.4,
            max_speed: 25.0,
            boost_max_speed: 50.0,
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
    pub roll_time: f32,
}

#[derive(Resource)]
pub struct CameraState {
    pub zoom: f32,
    pub orbit_yaw: f32,
    pub orbit_pitch: f32,
    pub orbiting: bool,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            zoom: 0.0,
            orbit_yaw: 0.0,
            orbit_pitch: 0.0,
            orbiting: false,
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
    let bottom_contact = grounded && car_down.dot(raw_normal) < -0.7;

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

    let steer_strength = (forward_speed.abs() / config.steer_speed_response).clamp(0.0, 1.0);
    let steer_sign = if forward_speed < 0.0 { -1.0 } else { 1.0 };
    if steer_strength > 0.01 && bottom_contact {
        forces.apply_torque(car_rot * Vec3::Y * input.steer * steer_sign * config.steer_torque * steer_strength);
    }

    if input.roll.abs() > 0.01 {
        forces.apply_torque(car_rot * Vec3::Z * input.roll * 600.0);
    }

    forces.apply_force(-velocity * config.rolling_resistance);
    forces.apply_force(-velocity * speed * config.drag_coefficient);

    let effective_max = if input.boosting { config.boost_max_speed } else { config.max_speed };
    if speed > effective_max {
        let excess = speed - effective_max;
        forces.apply_force(-velocity.normalize_or(Vec3::ZERO) * excess * 200.0);
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

fn clamp_speed(
    input: Res<CarInput>,
    config: Res<VehiclePhysicsConfig>,
    mut vel_query: Query<&mut LinearVelocity, With<PlayerCar>>,
) {
    let Ok(mut vel) = vel_query.single_mut() else {
        return;
    };
    let speed = vel.length();
    let effective_max = if input.boosting { config.boost_max_speed } else { config.max_speed };
    if speed > effective_max {
        vel.0 = vel.0.normalize_or(Vec3::ZERO) * effective_max;
    }
}

fn capture_input(time: Res<Time>, keys: Res<ButtonInput<KeyCode>>, mut input: ResMut<CarInput>) {
    input.throttle = if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        1.0
    } else if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        -0.5
    } else {
        0.0
    };

    let left = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
    let right = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight);
    input.steer = match (left, right) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
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

    if input.roll.abs() > 0.01 {
        input.roll_time += time.delta_secs();
    } else {
        input.roll_time = 0.0;
    }
}

fn sync_car_state(
    input: Res<CarInput>,
    mut car_query: Query<
        (&LinearVelocity, &Rotation, &Position, &mut Car),
        With<PlayerCar>,
    >,
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

    car_state.prev_speed = car_state.speed;
    car_state.speed = forward_speed;
    car_state.yaw = yaw;
    car_state.position = position.0;
    car_state.braking = input.braking;
    car_state.boosting = input.boosting;
}

fn camera_input(
    mut cam_state: ResMut<CameraState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut scroll_events: MessageReader<MouseWheel>,
    mut motion_events: MessageReader<MouseMotion>,
    car_query: Query<&Position, With<PlayerCar>>,
    cam_query: Query<&Transform, (With<CarCamera>, Without<PlayerCar>)>,
) {
    for ev in scroll_events.read() {
        cam_state.zoom = (cam_state.zoom + ev.y * 0.5).clamp(0.0, 12.0);
    }

    if mouse_buttons.just_pressed(MouseButton::Right) {
        if let Ok(car_pos) = car_query.single() {
            if let Ok(cam) = cam_query.single() {
                let car_center = car_pos.0 + Vec3::new(0.0, 1.5, 0.0);
                let offset = cam.translation - car_center;
                let horiz_dist = Vec2::new(offset.x, offset.z).length();
                cam_state.orbit_yaw = offset.x.atan2(offset.z);
                cam_state.orbit_pitch = offset.y.atan2(horiz_dist);
            }
        }
        cam_state.orbiting = true;
    }

    if cam_state.orbiting {
        for ev in motion_events.read() {
            cam_state.orbit_yaw -= ev.delta.x * 0.005;
            cam_state.orbit_pitch -= ev.delta.y * 0.005;
        }
        cam_state.orbit_pitch = cam_state.orbit_pitch.clamp(0.05, 1.45);
    } else {
        for _ in motion_events.read() {}
    }

    if mouse_buttons.just_released(MouseButton::Right) {
        cam_state.orbiting = false;
    }
}

fn camera_follow(
    car_query: Query<(&Rotation, &Position), With<PlayerCar>>,
    mut cam_query: Query<&mut Transform, (With<CarCamera>, Without<PlayerCar>)>,
    cam_state: Res<CameraState>,
) {
    let Some((car_rot, car_pos)) = car_query.iter().next() else {
        return;
    };

    let car_center = car_pos.0 + Vec3::new(0.0, 1.5, 0.0);
    let distance = (14.0 - cam_state.zoom).max(2.0);

    let target = if cam_state.orbiting {
        let horiz_dist = distance * cam_state.orbit_pitch.cos();
        let x = horiz_dist * cam_state.orbit_yaw.sin();
        let y = distance * cam_state.orbit_pitch.sin();
        let z = horiz_dist * cam_state.orbit_yaw.cos();
        car_center + Vec3::new(x, y, z)
    } else {
        let facing = *car_rot * Vec3::Z;
        let flat = Vec3::new(facing.x, 0.0, facing.z).normalize_or(Vec3::Z);
        let chase_height = (8.0 - cam_state.zoom * 0.2).max(2.0);
        car_pos.0 - flat * distance + Vec3::new(0.0, chase_height, 0.0)
    };

    let lerp = if cam_state.orbiting { 0.3 } else { 0.08 };

    for mut cam in cam_query.iter_mut() {
        cam.translation = cam.translation.lerp(target, lerp);
        cam.look_at(car_center, Vec3::Y);
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
                let lower = name.to_lowercase().replace(['-', '_'], "");
                match lower.as_str() {
                    "wheelfrontleft" | "fl" => {
                        commands.entity(child).insert(WheelFrontLeft);
                        found_wheels = true;
                    }
                    "wheelfrontright" | "fr" => {
                        commands.entity(child).insert(WheelFrontRight);
                        found_wheels = true;
                    }
                    "wheelbackleft" | "wheelrearleft" | "bl" => {
                        commands.entity(child).insert(WheelRearLeft);
                        found_wheels = true;
                    }
                    "wheelbackright" | "wheelrearright" | "br" => {
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
) {
    let Some(_car) = car_data.iter().next() else {
        return;
    };
    let dt = time.delta_secs();

    let skid_mult = if car_state.braking { 0.4 } else { 1.0 };
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

#[derive(Component)]
pub struct RespawnCar {
    pub spawn_pos: Vec3,
}

fn respawn_car(
    mut player_query: Query<(&mut Position, &mut Rotation, &mut LinearVelocity, &mut AngularVelocity), With<PlayerCar>>,
    mut ai_query: Query<(&mut Position, &mut Rotation, &mut LinearVelocity, &mut AngularVelocity), (With<AiCar>, Without<PlayerCar>)>,
) {
    for (mut pos, mut rot, mut lin_vel, mut ang_vel) in player_query.iter_mut() {
        if pos.0.y < -20.0 {
            pos.0 = Vec3::new(0.0, 5.0, 0.0);
            rot.0 = Quat::IDENTITY;
            lin_vel.0 = Vec3::ZERO;
            ang_vel.0 = Vec3::ZERO;
        }
    }
    let mut ai_index: u32 = 0;
    for (mut pos, mut rot, mut lin_vel, mut ang_vel) in ai_query.iter_mut() {
        if pos.0.y < -20.0 {
            let angle = ai_index as f32 * 2.1;
            pos.0 = Vec3::new(angle.cos() * 40.0, 5.0, angle.sin() * 40.0);
            rot.0 = Quat::IDENTITY;
            lin_vel.0 = Vec3::ZERO;
            ang_vel.0 = Vec3::ZERO;
            ai_index += 1;
        }
    }
}

fn respawn_hit_cars(
    mut commands: Commands,
    query: Query<(Entity, &RespawnCar)>,
    mut pos_query: Query<&mut Position>,
    mut rot_query: Query<&mut Rotation>,
    mut linvel_query: Query<&mut LinearVelocity>,
    mut angvel_query: Query<&mut AngularVelocity>,
) {
    for (entity, respawn) in query.iter() {
        if let Ok(mut pos) = pos_query.get_mut(entity) {
            pos.0 = respawn.spawn_pos;
        }
        if let Ok(mut rot) = rot_query.get_mut(entity) {
            rot.0 = Quat::IDENTITY;
        }
        if let Ok(mut lin_vel) = linvel_query.get_mut(entity) {
            lin_vel.0 = Vec3::ZERO;
        }
        if let Ok(mut ang_vel) = angvel_query.get_mut(entity) {
            ang_vel.0 = Vec3::ZERO;
        }
        commands.entity(entity).remove::<RespawnCar>();
    }
}

#[derive(Component)]
pub struct CarVisual;

#[derive(Component)]
pub struct CarCollider;

fn switch_car_model(
    mut selection: ResMut<CarSelection>,
    car_query: Query<Entity, With<PlayerCar>>,
    children_query: Query<&Children>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    blaster_selection: Res<crate::blaster::BlasterSelection>,
) {
    if !selection.pending_change {
        return;
    }
    selection.pending_change = false;

    let Ok(car_entity) = car_query.single() else {
        return;
    };

    for child in children_query.iter_descendants(car_entity) {
        commands.entity(child).despawn();
    }

    commands.entity(car_entity).remove::<WheelsLabeled>();

    let def = &CAR_DEFS[selection.index];
    let car_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(def.path));
    let half_height = def.collider.y * 0.5;
    let mount_y = crate::car::mount_y(def.collider.y);
    let blaster_def = &crate::blaster::BLASTER_DEFS[blaster_selection.index];
    let blaster_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(blaster_def.path));

    commands.entity(car_entity).with_children(|parent| {
        parent.spawn((
            Collider::cuboid(def.collider.x, def.collider.y, def.collider.z),
            Transform::from_translation(Vec3::new(0.0, half_height, 0.0)),
            CollisionLayers::new(LayerMask(0b010), LayerMask(0xFFFFFFFF)),
            CarCollider,
        ));
        parent.spawn((SceneRoot(car_scene), CarVisual));
        parent.spawn((
            SceneRoot(blaster_scene),
            Transform::from_translation(Vec3::new(0.0, mount_y, 0.0))
                .with_scale(Vec3::splat(blaster_def.scale))
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
            crate::blaster::BlasterVisual,
            crate::blaster::ComputePivot,
        ));
    });
}

fn switch_car_model_pregame(
    mut car_selection: ResMut<CarSelection>,
    mut blaster_selection: ResMut<crate::blaster::BlasterSelection>,
    car_query: Query<Entity, With<PlayerCar>>,
    children_query: Query<&Children>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if !car_selection.pending_change && !blaster_selection.pending_change {
        return;
    }
    car_selection.pending_change = false;
    blaster_selection.pending_change = false;

    let Ok(car_entity) = car_query.single() else {
        return;
    };

    for child in children_query.iter_descendants(car_entity) {
        commands.entity(child).despawn();
    }

    commands.entity(car_entity).remove::<WheelsLabeled>();

    let def = &CAR_DEFS[car_selection.index];
    let car_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(def.path));
    let half_height = def.collider.y * 0.5;
    let mount_y = crate::car::mount_y(def.collider.y);
    let blaster_def = &crate::blaster::BLASTER_DEFS[blaster_selection.index];
    let blaster_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(blaster_def.path));

    commands.entity(car_entity).with_children(|parent| {
        parent.spawn((
            Collider::cuboid(def.collider.x, def.collider.y, def.collider.z),
            Transform::from_translation(Vec3::new(0.0, half_height, 0.0)),
            CollisionLayers::new(LayerMask(0b010), LayerMask(0xFFFFFFFF)),
            CarCollider,
        ));
        parent.spawn((SceneRoot(car_scene), CarVisual));
        parent.spawn((
            SceneRoot(blaster_scene),
            Transform::from_translation(Vec3::new(0.0, mount_y, 0.0))
                .with_scale(Vec3::splat(blaster_def.scale))
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
            crate::blaster::BlasterVisual,
            crate::blaster::ComputePivot,
        ));
    });
}