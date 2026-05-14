use bevy::prelude::*;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, _app: &mut App) {
        // Placeholder for Rapier integration
        // Will handle:
        // - Collision detection
        // - Vehicle physics (suspension, tire friction)
        // - Deterministic simulation mode
    }
}

#[derive(Component)]
pub struct PhysicsBody {
    pub mass: f32,
    pub drag: f32,
    pub friction: f32,
}

impl Default for PhysicsBody {
    fn default() -> Self {
        Self {
            mass: 1000.0,
            drag: 0.3,
            friction: 0.8,
        }
    }
}
