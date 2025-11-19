use bevy_ecs::prelude::*;
use crate::components::{Position, VehicleId, Velocity}; // Убрали Route
use traffic_common::proto::traffic::{VehicleUpdate, TrafficUpdate};
use prost::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;

// Обертка для продюсера Kafka, чтобы хранить его в ECS World
#[derive(Resource)]
pub struct KafkaProducer(pub FutureProducer);

// Счетчик для отправки не каждый кадр (оптимизация)
#[derive(Resource)]
pub struct BroadcastCounter(pub u32);

pub fn broadcast_system(
    mut counter: ResMut<BroadcastCounter>,
    producer: Res<KafkaProducer>,
    // Читаем только то, что есть: ID, Позицию, Скорость
    query: Query<(&VehicleId, &Position, &Velocity)>,
) {
    // Отправляем обновления только каждый 5-й кадр (примерно 12 раз в секунду при 60fps)
    // Это снижает нагрузку на сеть и Kafka
    counter.0 += 1;
    if counter.0 < 5 {
        return;
    }
    counter.0 = 0;

    let mut updates = Vec::with_capacity(query.iter().len());

    for (vid, pos, vel) in query.iter() {
        updates.push(VehicleUpdate {
            vehicle_id: vid.0.clone(),
            // Переводим координаты в формат protobuf
            latitude: pos.0.y as f64,
            longitude: pos.0.x as f64,
            speed: vel.0.length(), // Длина вектора скорости
            heading: 0.0, // Пока заглушка, вычислим позже из вектора
        });
    }

    if updates.is_empty() {
        return;
    }

    // Формируем пакет
    let traffic_update = TrafficUpdate {
        vehicles: updates,
        timestamp: 0, // Можно добавить реальное время
    };

    // Сериализация в байты
    let mut payload = Vec::new();
    if let Err(e) = traffic_update.encode(&mut payload) {
        tracing::error!("Failed to encode protobuf: {:?}", e);
        return;
    }

    // Отправка в Kafka (Fire and Forget)
    let record = FutureRecord::to("traffic.updates")
        .payload(&payload)
        .key("berlin_center"); // Ключ для партиционирования

    // Мы не ждем результата (await) внутри синхронной системы ECS,
    // библиотека rdkafka отправит это асинхронно в фоне.
    let _ = producer.0.send(record, Duration::from_secs(0));
}