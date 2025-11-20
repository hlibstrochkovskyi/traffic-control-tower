//! ECS systems for vehicle movement and position synchronization.
//!
//! This module contains the core simulation logic for moving vehicles along
//! the road network graph and synchronizing their visual positions.

use bevy_ecs::prelude::*;
use crate::components::*;
use traffic_common::map::RoadGraph;
use glam::Vec2;

/// Updates vehicle positions along road network edges based on their speed.
///
/// This system moves vehicles along their current road segment, advancing them
/// based on their target speed and elapsed time. When a vehicle reaches the end
/// of a road segment, it randomly selects the next connected road to continue on.
///
/// # Behavior
///
/// - Advances each vehicle along its current road edge
/// - Handles road transitions when reaching the end of a segment
/// - Randomly selects next road from available outgoing edges
/// - Stops vehicles that reach dead ends
///
/// # Parameters
///
/// * `time` - Delta time resource for frame-independent movement
/// * `graph` - Road network graph containing road segments and topology
/// * `query` - Query for all entities with graph position and target speed
pub fn movement_system(
    time: Res<DeltaTime>,
    graph: Res<RoadGraph>,
    mut query: Query<(&mut GraphPosition, &TargetSpeed)>,
) {
    for (mut graph_pos, target_speed) in query.iter_mut() {
        // Get the current road segment
        if let Some(road) = graph.edges.get(graph_pos.edge_index) {
            // Move along the road
            let speed_m_per_sec = target_speed.0 as f64;
            graph_pos.distance += speed_m_per_sec * (time.0 as f64);

            // Check if we've reached the end of the current road
            if graph_pos.distance >= road.length {
                // Look for outgoing roads from the end of the current road
                if let Some(next_edges) = graph.out_edges.get(&road.end) {
                    if !next_edges.is_empty() {
                        // Randomly select the next road
                        let next_idx = next_edges[rand::random::<usize>() % next_edges.len()];
                        graph_pos.edge_index = next_idx;
                        graph_pos.distance = 0.0;
                    } else {
                        // Dead end - stop at the end of the road
                        graph_pos.distance = road.length;
                    }
                } else {
                    // No outgoing roads - stop here
                    graph_pos.distance = road.length;
                }
            }
        }
    }
}

/// Synchronizes visual positions with graph-based logical positions.
///
/// This system converts abstract graph positions (edge index + distance)
/// into concrete 2D coordinates for rendering. It handles both simple
/// straight road segments and complex curved roads with multiple geometry points.
///
/// # Parameters
///
/// * `graph` - Road network graph with geometric road data
/// * `query` - Query for all entities with both graph and visual positions
pub fn sync_position_system(
    graph: Res<RoadGraph>,
    mut query: Query<(&GraphPosition, &mut Position)>,
) {
    for (graph_pos, mut pos) in query.iter_mut() {
        if let Some(road) = graph.edges.get(graph_pos.edge_index) {
            if road.geometry.len() >= 2 {
                // Calculate progress along the road (0.0 to 1.0)
                let progress = (graph_pos.distance / road.length).clamp(0.0, 1.0);

                // For roads with only 2 points (simple segment), do linear interpolation
                if road.geometry.len() == 2 {
                    let start = road.geometry[0];
                    let end = road.geometry[1];
                    let interpolated = start + (end - start) * progress;
                    pos.0 = Vec2::new(interpolated.x as f32, interpolated.y as f32);
                } else {
                    // For roads with multiple geometry points, interpolate along the polyline
                    // This provides smooth movement along curved roads
                    let interpolated = interpolate_along_polyline(&road.geometry, progress);
                    pos.0 = Vec2::new(interpolated.x as f32, interpolated.y as f32);
                }
            }
        }
    }
}

/// Interpolates a position along a polyline based on normalized progress.
///
/// For curved roads represented by multiple points, this function calculates
/// the exact position at a given progress value (0.0 = start, 1.0 = end)
/// by finding the appropriate segment and interpolating within it.
///
/// # Arguments
///
/// * `geometry` - Sequence of points defining the road's shape
/// * `progress` - Normalized distance along the road (0.0 to 1.0)
///
/// # Returns
///
/// The interpolated 2D position along the polyline.
///
/// # Algorithm
///
/// 1. Calculates total polyline length
/// 2. Determines which segment contains the target distance
/// 3. Performs linear interpolation within that segment
fn interpolate_along_polyline(geometry: &[glam::DVec2], progress: f64) -> glam::DVec2 {
    if geometry.len() < 2 {
        return geometry[0];
    }

    // Calculate total length of the polyline
    let mut segment_lengths = Vec::new();
    let mut total_length = 0.0;

    for i in 0..geometry.len() - 1 {
        let len = (geometry[i + 1] - geometry[i]).length();
        segment_lengths.push(len);
        total_length += len;
    }

    if total_length == 0.0 {
        return geometry[0];
    }

    // Find which segment we're on based on progress
    let target_distance = progress * total_length;
    let mut accumulated_distance = 0.0;

    for (i, &seg_len) in segment_lengths.iter().enumerate() {
        if accumulated_distance + seg_len >= target_distance {
            // We're on this segment
            let segment_progress = if seg_len > 0.0 {
                (target_distance - accumulated_distance) / seg_len
            } else {
                0.0
            };

            // Linear interpolation within this segment
            let start = geometry[i];
            let end = geometry[i + 1];
            return start + (end - start) * segment_progress;
        }
        accumulated_distance += seg_len;
    }

    // If we get here, return the last point
    geometry[geometry.len() - 1]
}