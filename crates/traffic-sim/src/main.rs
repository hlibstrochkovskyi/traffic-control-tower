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

    // 1. Загружаем Карту
    let map_path = "crates/traffic-sim/assets/berlin.osm.pbf";
    let road_graph = RoadGraph::load_from_pbf(map_path)?;

    // 2. Инициализация ресурсов
    world.insert_resource(DeltaTime(1.0 / 60.0));
    world.insert_resource(BroadcastCounter(0));

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &config.kafka_brokers)
        .set("message.timeout.ms", "5000")
        .create()?;
    world.insert_resource(KafkaProducer(producer));

    // 3. Настройка систем
    let mut schedule = Schedule::default();
    schedule.add_systems((
        movement_system,      // ← Система движения
        sync_position_system, // ← Синхронизация графовой и визуальной позиции
        broadcast_system,     // ← Отправка данных в Kafka
    ));

    // 4. Спавним машины (передаем граф явно как аргумент)
    spawn_vehicles_on_graph(&mut world, &road_graph, 5000);

    // 5. Теперь отдаем карту миру (после спавна она нам в main больше не нужна)
    world.insert_resource(road_graph);

    tracing::info!("🚀 Simulation loop starting...");

    let mut last_tick = Instant::now();
    let target_frametime = Duration::from_millis(16); // 60 FPS

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

// Спавн машин на случайных дорогах
fn spawn_vehicles_on_graph(world: &mut World, graph: &RoadGraph, count: usize) {
    let mut rng = rand::thread_rng();
    let edge_count = graph.edges.len();

    if edge_count == 0 {
        tracing::error!("Zero roads found! Cannot spawn vehicles.");
        return;
    }

    tracing::info!("🅿️ Spawning {} vehicles on random roads...", count);

    for i in 0..count {
        // 1. Выбираем случайную дорогу
        let edge_idx = rng.gen_range(0..edge_count);
        let road = &graph.edges[edge_idx];

        if road.geometry.is_empty() {
            continue;
        }

        // 2. Ставим машину в начало этой дороги
        let start_pos = road.geometry[0];

        world.spawn((
            VehicleId(format!("car_{}", i)),

            // Графическая позиция (для фронта)
            Position(Vec2::new(start_pos.x as f32, start_pos.y as f32)),

            // Логическая позиция (для физики)
            GraphPosition {
                edge_index: edge_idx,
                distance: 0.0, // В начале сегмента
            },

            Velocity(Vec2::ZERO), // Пока стоят
            TargetSpeed(rng.gen_range(10.0..20.0)),
        ));
    }

    tracing::info!("✅ {} vehicles spawned.", count);
}