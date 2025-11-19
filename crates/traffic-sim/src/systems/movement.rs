use bevy_ecs::prelude::*;
use crate::components::*;
// use glam::Vec2; // You can remove this if the IDE warns about an unused import, or keep it

#[derive(Resource)]
pub struct DeltaTime(pub f32);

pub fn movement_system(
    time: Res<DeltaTime>,
    mut query: Query<(&mut Position, &Velocity)>,
) {
    query.par_iter_mut().for_each(|(mut pos, vel)| {
        pos.0 += vel.0 * time.0;
    });
}

pub fn steering_system(
    mut query: Query<(&Position, &mut Velocity, &Route, &TargetSpeed)>,
) {
    query.par_iter_mut().for_each(|(pos, mut vel, route, target_speed)| {
        if let Some(waypoint) = route.waypoints.get(route.current_waypoint) {
            let direction = (*waypoint - pos.0).normalize_or_zero();
            let desired_velocity = direction * target_speed.0;

            // FIX: store current velocity in a variable
            let current_velocity = vel.0;

            // Now the formula works because we don't read from vel inside the assignment expression
            vel.0 += (desired_velocity - current_velocity) * 0.1;
        }
    });
}

pub fn waypoint_system(
    mut query: Query<(&Position, &mut Route)>,
) {
    const WAYPOINT_THRESHOLD: f32 = 0.0002;

    query.par_iter_mut().for_each(|(pos, mut route)| {
        if let Some(waypoint) = route.waypoints.get(route.current_waypoint) {
            if pos.0.distance(*waypoint) < WAYPOINT_THRESHOLD {
                route.current_waypoint = (route.current_waypoint + 1) % route.waypoints.len();
            }
        }
    });
}