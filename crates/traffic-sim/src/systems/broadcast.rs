use bevy_ecs::prelude::*;
use traffic_common::VehiclePosition;
use rdkafka::producer::FutureProducer;
use prost::Message;

#[derive(Resource)]
pub struct KafkaProducer(pub FutureProducer);

#[derive(Resource)]
pub struct BroadcastCounter(pub u32);

pub fn broadcast_system(
    query: Query<(&crate::components::VehicleId, &crate::components::Position, &crate::components::Velocity)>,
    producer: Res<KafkaProducer>,
    mut counter: ResMut<BroadcastCounter>,
) {
    counter.0 += 1;
    // Send less frequently (once every 60 frames) to avoid network overload
    if counter.0 < 60 {
        return;
    }
    counter.0 = 0;

    for (id, pos, vel) in query.iter() {
        let msg = VehiclePosition {
            vehicle_id: id.0.clone(),
            latitude: pos.0.y as f64,
            longitude: pos.0.x as f64,
            speed: vel.0.length() as f64,
            timestamp: chrono::Utc::now().timestamp(),
        };

        let mut buf = Vec::new();
        if msg.encode(&mut buf).is_ok() {
            // 1. Clone the producer (it's cheap; internally an Arc)
            let producer_clone = producer.0.clone();

            // 2. Prepare the key (we need ownership of the string)
            let key = msg.vehicle_id.clone();

            // 3. Fire and forget
            // move forces capturing buf and key into the task
            tokio::spawn(async move {
                // IMPORTANT: Create the Record INSIDE the task.
                // Now it references buf and key which were moved here.
                let record = rdkafka::producer::FutureRecord::to("raw-telemetry")
                    .payload(&buf)
                    .key(&key);

                let _ = producer_clone.send(record, std::time::Duration::from_secs(0)).await;
            });
        }
    }
}