mod components;
mod systems;
mod routes;

use bevy_ecs::prelude::*;
use components::*;
use systems::movement::*;
use systems::broadcast::*;
use traffic_common::{Config, init_tracing};
use glam::Vec2;
use std::time::{Duration, Instant};
use rdkafka::config::ClientConfig;
use rdkafka::producer::FutureProducer;
use anyhow::{Context, Result};
use routes::berlin_ring_route;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing("traffic-sim");
    let config = Config::from_env()?;

    let mut world = World::new();
    let mut schedule = Schedule::default();

    world.insert_resource(DeltaTime(1.0 / 60.0));
    world.insert_resource(BroadcastCounter(0));

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &config.kafka_brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .context("Failed to create Kafka producer")?;
    world.insert_resource(KafkaProducer(producer));

    schedule.add_systems((
        steering_system,
        movement_system.after(steering_system),
        waypoint_system.after(movement_system),
        broadcast_system.after(waypoint_system),
    ));

    // === DEBUG SCENARIO ===
    tracing::info!("🧪 Starting DEBUG mode: 50 cars total");

    // Группа 1: СИНИЕ (Кольцо) - 25 машин
    // Скорость 0.0003 (медленные)
    spawn_group(&mut world, 25, berlin_ring_route(), 0.0003, "ring");

    // Группа 2: КРАСНЫЕ (Линия) - 25 машин
    // Прямая линия через центр: от 13.30 до 13.50 по широте 52.52
    let line_route = vec![
        Vec2::new(13.30, 52.52),
        Vec2::new(13.50, 52.52),
        Vec2::new(13.30, 52.52), // Обратно
    ];
    // Скорость 0.0008 (быстрые)
    spawn_group(&mut world, 25, line_route, 0.0008, "line");

    tracing::info!("✅ Debug vehicles spawned. Look for RED line and BLUE ring.");

    let mut last_tick = Instant::now();
    let target_frametime = Duration::from_millis(16);

    loop {
        let now = Instant::now();
        let delta = (now - last_tick).as_secs_f32();
        last_tick = now;
        *world.resource_mut::<DeltaTime>() = DeltaTime(delta);
        schedule.run(&mut world);
        let elapsed = Instant::now() - now;
        if elapsed < target_frametime {
            tokio::time::sleep(target_frametime - elapsed).await;
        }
    }
}

fn spawn_group(world: &mut World, count: usize, route: Vec<Vec2>, speed: f32, prefix: &str) {
    for i in 0..count {
        // Равномерно распределяем по маршруту
        let wp_idx = i % route.len();
        let start_pos = route[wp_idx];

        world.spawn((
            VehicleId(format!("{}_{}", prefix, i)),
            Position(start_pos),
            Velocity(Vec2::ZERO),
            Route {
                waypoints: route.clone(),
                current_waypoint: (wp_idx + 1) % route.len(),
            },
            TargetSpeed(speed),
        ));
    }
}