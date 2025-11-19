mod components;
mod systems;
mod routes;

use bevy_ecs::prelude::*;
use components::*;
use systems::movement::*;
use systems::broadcast::*;
use routes::berlin_ring_route;
use traffic_common::{Config, init_tracing};
use glam::Vec2;
use std::time::{Duration, Instant};
use rdkafka::config::ClientConfig;
use rdkafka::producer::FutureProducer;
use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing("traffic-sim");
    let config = Config::from_env()?;

    // 1. ECS setup
    let mut world = World::new();
    let mut schedule = Schedule::default();

    // 2. Resources (global variables)
    // DeltaTime: time between frames (for physics)
    world.insert_resource(DeltaTime(1.0 / 60.0));
    // BroadcastCounter: to avoid sending to Kafka every frame
    world.insert_resource(BroadcastCounter(0));

    // Kafka Producer
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &config.kafka_brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .context("Failed to create Kafka producer")?;
    world.insert_resource(KafkaProducer(producer));

    // 3. Systems (logic that runs each frame)
    schedule.add_systems((
        steering_system,                         // 1. Steer to the target
        movement_system.after(steering_system),  // 2. Move
        waypoint_system.after(movement_system),  // 3. Check if arrived
        broadcast_system.after(waypoint_system), // 4. Send to Kafka
    ));

    // 4. Spawn vehicles (5000 of them!)
    tracing::info!("Spawning 5000 vehicles...");
    spawn_vehicles(&mut world, 5000);
    tracing::info!("Simulation started. Press Ctrl+C to stop.");

    // 5. Main loop (game loop)
    let mut last_tick = Instant::now();
    let target_frametime = Duration::from_millis(16); // ~60 FPS

    loop {
        let now = Instant::now();
        // Calculate real dt (frame time)
        let delta = (now - last_tick).as_secs_f32();
        last_tick = now;

        // Update the time resource in ECS
        *world.resource_mut::<DeltaTime>() = DeltaTime(delta);

        // RUN ALL SYSTEMS
        schedule.run(&mut world);

        // FPS limiting (to avoid maxing the CPU unnecessarily)
        let elapsed = Instant::now() - now;
        if elapsed < target_frametime {
            tokio::time::sleep(target_frametime - elapsed).await;
        }
    }
}

fn spawn_vehicles(world: &mut World, count: usize) {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Get the shared Berlin ring route waypoints
    let waypoints = berlin_ring_route();
    let num_waypoints = waypoints.len();

    for i in 0..count {
        // Distribute vehicles evenly along the route (using modulo to cycle through waypoints)
        let waypoint_index = i % num_waypoints;
        let base_pos = waypoints[waypoint_index];
        
        // Add small random offset (±0.0001°) to avoid perfect overlap
        let start_pos = Vec2::new(
            base_pos.x + rng.gen_range(-0.0001..0.0001),
            base_pos.y + rng.gen_range(-0.0001..0.0001),
        );

        // Next waypoint in sequence (circular)
        let next_waypoint_index = (waypoint_index + 1) % num_waypoints;

        // Create an Entity with a set of Components
        world.spawn((
            VehicleId(format!("car_{}", i)),
            Position(start_pos),
            Velocity(Vec2::ZERO), // Initially stationary
            Route {
                waypoints: waypoints.clone(),
                current_waypoint: next_waypoint_index,
            },
            // Uniform speed (0.0008) for clear visualization
            TargetSpeed(0.0008),
        ));
    }
}