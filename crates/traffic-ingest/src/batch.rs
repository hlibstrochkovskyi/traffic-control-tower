use sqlx::PgPool;
use traffic_common::{VehiclePosition, Result};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct BatchWriter {
    pool: PgPool,
    buffer: Arc<Mutex<Vec<VehiclePosition>>>,
    batch_size: usize,
}

impl BatchWriter {
    pub fn new(pool: PgPool, batch_size: usize) -> Self {
        Self {
            pool,
            buffer: Arc::new(Mutex::new(Vec::with_capacity(batch_size))),
            batch_size,
        }
    }

    // Add a position to the buffer
    pub async fn add(&self, position: VehiclePosition) -> Result<()> {
        let mut buffer = self.buffer.lock().await;
        buffer.push(position);

        // If the buffer is full — flush it to the DB
        if buffer.len() >= self.batch_size {
            self.flush_locked(&mut buffer).await?;
        }
        Ok(())
    }

    // Internal write logic
    async fn flush_locked(&self, buffer: &mut Vec<VehiclePosition>) -> Result<()> {
        if buffer.is_empty() {
            return Ok(());
        }

        // The log we expect
        tracing::info!("Saved {} positions to DB", buffer.len());

        let mut tx = self.pool.begin().await?;

        for pos in buffer.iter() {
            sqlx::query!(
                r#"
                INSERT INTO vehicle_positions (time, vehicle_id, latitude, longitude, speed)
                VALUES (to_timestamp($1), $2, $3, $4, $5)
                "#,
                pos.timestamp as f64,
                pos.vehicle_id,
                pos.latitude,
                pos.longitude,
                pos.speed
            )
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        buffer.clear();
        Ok(())
    }

    // Forced flush (e.g., on shutdown)
    pub async fn flush(&self) -> Result<()> {
        let mut buffer = self.buffer.lock().await;
        self.flush_locked(&mut buffer).await
    }
}