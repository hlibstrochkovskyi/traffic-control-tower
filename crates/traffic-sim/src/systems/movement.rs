use bevy_ecs::prelude::*;
use crate::components::*;
use traffic_common::map::RoadGraph;
use glam::Vec2;

// Система движения по графу дорог
pub fn movement_system(
    time: Res<DeltaTime>,
    graph: Res<RoadGraph>,
    mut query: Query<(&mut GraphPosition, &TargetSpeed)>,
) {
    for (mut graph_pos, target_speed) in query.iter_mut() {
        // Получаем текущую дорогу
        if let Some(road) = graph.edges.get(graph_pos.edge_index) {
            // Движемся вдоль дороги
            let speed_m_per_sec = target_speed.0 as f64; // м/с, convert to f64
            graph_pos.distance += speed_m_per_sec * (time.0 as f64);

            // Если достигли конца дороги - переходим на следующую
            if graph_pos.distance >= road.length {
                // Смотрим, есть ли исходящие дороги из конца текущей
                if let Some(next_edges) = graph.out_edges.get(&road.end) {
                    if !next_edges.is_empty() {
                        // Выбираем случайную следующую дорогу
                        let next_idx = next_edges[rand::random::<usize>() % next_edges.len()];
                        graph_pos.edge_index = next_idx;
                        graph_pos.distance = 0.0;
                    } else {
                        // Тупик - останавливаемся в конце
                        graph_pos.distance = road.length;
                    }
                } else {
                    // Нет исходящих дорог - останавливаемся
                    graph_pos.distance = road.length;
                }
            }
        }
    }
}

// Синхронизация визуальной позиции с позицией на графе
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

// Helper function to interpolate along a polyline based on progress (0.0 to 1.0)
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