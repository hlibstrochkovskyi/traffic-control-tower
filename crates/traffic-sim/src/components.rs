//! ECS components and resources for the traffic simulation.
//!
//! This module defines the core data structures used in the Entity Component System
//! for vehicle simulation, including both global resources and per-entity components.

use bevy_ecs::prelude::*;
use glam::Vec2;

// --- RESOURCES (Global simulation data) ---

/// Delta time resource tracking elapsed time between simulation frames.
///
/// Used for frame-independent movement calculations. The value represents
/// the time in seconds since the last frame, potentially scaled for time acceleration.
#[derive(Resource, Debug, Clone, Copy)]
pub struct DeltaTime(pub f32);

// --- COMPONENTS (Per-vehicle data) ---

/// Unique identifier for a vehicle entity.
///
/// Each vehicle is assigned a unique string ID (e.g., "car_42") for
/// tracking and telemetry purposes.
#[derive(Component, Debug, Clone)]
pub struct VehicleId(pub String);

/// Visual 2D position of a vehicle in geographic coordinates.
///
/// Represents the vehicle's location on the map where:
/// - `x` = longitude
/// - `y` = latitude
///
/// This position is derived from the vehicle's graph position and used
/// for rendering and telemetry broadcasting.
#[derive(Component, Debug, Clone, Copy)]
pub struct Position(pub Vec2);

/// Velocity vector of a vehicle.
///
/// Represents the vehicle's current movement direction and speed.
/// Currently used for physics simulation and may be extended for
/// collision detection or acceleration modeling.
#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity(pub Vec2);

/// Logical position of a vehicle on the road network graph.
///
/// This component tracks which road segment a vehicle is currently on
/// and how far along that segment it has traveled. This abstract
/// representation is converted to geographic coordinates during rendering.
#[derive(Component, Debug, Clone)]
pub struct GraphPosition {
    /// Index of the current road edge in the road graph
    pub edge_index: usize,
    /// Distance traveled along the current edge in meters (0.0 to edge.length)
    pub distance: f64,
}

/// Target speed component defining a vehicle's desired velocity.
///
/// Represents the speed the vehicle aims to maintain in meters per second.
/// This is used by the movement system to advance vehicles along their roads.
///
/// Typical values range from 10.0 to 20.0 m/s (~36-72 km/h).
#[derive(Component, Debug, Clone, Copy)]
pub struct TargetSpeed(pub f32);