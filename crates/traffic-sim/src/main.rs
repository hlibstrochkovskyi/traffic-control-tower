//! Traffic Simulation Service - ECS-based vehicle movement simulator.
//!
//! This service simulates realistic vehicle movement on a road network using
//! the Bevy ECS framework. It spawns vehicles on the road graph, simulates
//! their movement, and broadcasts position updates to Kafka for downstream
//! processing.

mod components;
mod systems;

use bevy_ecs::prelude::*;
use components::*;
use systems::movement::*;
use systems::broadcast::*;
use traffic_common::{init_tracing, Config};
use traffic_common::map::RoadGraph;
use glam::Vec2;
use rand::Rng;
use std::time::{Duration, Instant};
use anyhow::Result;
use rdkafka::config::ClientConfig;
use rdkafka::producer::FutureProducer;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing("traffic-sim");
    let config = Config::from_env()?;

    let mut world = World::new();

    // Load the road network map
    let map_path = "crates/traffic-sim/assets/berlin.osm.pbf";
    let road_graph = RoadGraph::load_from_pbf(map_path)?;

    // Initialize ECS resources
    world.insert_resource(DeltaTime(1.0 / 60.0));
    world.insert_resource(BroadcastCounter(0));

    // Create Kafka producer for telemetry broadcasting
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &config.kafka_brokers)
        .set("message.timeout.ms", "5000")
        .create()?;
    world.insert_resource(KafkaProducer(producer));

    // Configure ECS system schedule
    let mut schedule = Schedule::default();
    schedule.add_systems((
        movement_system,      // Vehicle movement along roads
        sync_position_system, // Synchronize graph position to visual position
        broadcast_system,     // Send telemetry to Kafka
    ));

    // Spawn vehicles on the road network (before inserting graph as resource)
    spawn_vehicles_on_graph(&mut world, &road_graph, 5000);

    // Insert road graph as ECS resource after spawning
    world.insert_resource(road_graph);

    tracing::info!("🚀 Simulation loop starting...");

    let mut last_tick = Instant::now();
    let target_frametime = Duration::from_millis(16); // 60 FPS

    // Main simulation loop
    loop {
        let now = Instant::now();
        let delta = (now - last_tick).as_secs_f32();
        last_tick = now;

        // Apply time acceleration (10x real-time)
        let time_scale = 10.0;
        *world.resource_mut::<DeltaTime>() = DeltaTime(delta * time_scale);

        // Execute all systems
        schedule.run(&mut world);

        // Maintain consistent frame rate
        let elapsed = Instant::now() - now;
        if elapsed < target_frametime {
            tokio::time::sleep(target_frametime - elapsed).await;
        }
    }
}

/// Spawns vehicles at random positions on the road network.
///
/// Each vehicle is placed at the start of a randomly selected road segment
/// with a random target speed. The vehicles are assigned unique IDs and
/// initialized with both visual and graph-based positions.
///
/// # Arguments
///
/// * `world` - The ECS world to spawn entities into
/// * `graph` - Road network graph (passed separately before becoming a resource)
/// * `count` - Number of vehicles to spawn
///
/// # Behavior
///
/// - Randomly selects road segments for each vehicle
/// - Places vehicles at the start of their assigned road
/// - Assigns random speeds between 10-20 m/s
/// - Skips roads with no geometry data
fn spawn_vehicles_on_graph(world: &mut World, graph: &RoadGraph, count: usize) {
    let mut rng = rand::thread_rng();
    let edge_count = graph.edges.len();

    if edge_count == 0 {
        tracing::error!("Zero roads found! Cannot spawn vehicles.");
        return;
    }

    tracing::info!("🅿️ Spawning {} vehicles on random roads...", count);

    for i in 0..count {
        // Select a random road segment
        let edge_idx = rng.gen_range(0..edge_count);
        let road = &graph.edges[edge_idx];

        if road.geometry.is_empty() {
            continue;
        }

        // Place vehicle at the start of the road
        let start_pos = road.geometry[0];

        world.spawn((
            VehicleId(format!("car_{}", i)),

            // Visual position for frontend rendering
            Position(Vec2::new(start_pos.x as f32, start_pos.y as f32)),

            // Logical position on the road graph
            GraphPosition {
                edge_index: edge_idx,
                distance: 0.0, // At the start of the segment
            },

            Velocity(Vec2::ZERO), // Initially stationary
            TargetSpeed(rng.gen_range(10.0..20.0)), // Random speed in m/s
        ));
    }

    tracing::info!("✅ {} vehicles spawned.", count);
}