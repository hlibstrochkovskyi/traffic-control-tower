use bevy_ecs::prelude::*;
use glam::Vec2;

// Vehicle position in 2D space (x=longitude, y=latitude for simplicity)
#[derive(Component, Debug, Clone, Copy)]
pub struct Position(pub Vec2);

// Velocity vector (direction and speed)
#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity(pub Vec2);

// Unique ID for Kafka (e.g., "car_42")
#[derive(Component, Debug, Clone)]
pub struct VehicleId(pub String);

// Route: list of points to traverse
#[derive(Component, Debug, Clone)]
pub struct Route {
    pub waypoints: Vec<Vec2>,
    pub current_waypoint: usize,
}

// Desired speed (driver may want, e.g., 60 km/h, but can slow down)
#[derive(Component, Debug, Clone, Copy)]
pub struct TargetSpeed(pub f32);