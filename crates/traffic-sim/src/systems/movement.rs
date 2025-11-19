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
            let speed_m_per_sec = target_speed.0; // м/с
            graph_pos.distance += speed_m_per_sec * time.0;

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
                // Интерполируем позицию вдоль геометрии дороги
                let progress = (graph_pos.distance / road.length).clamp(0.0, 1.0);

                // Простая линейная интерполяция между началом и концом
                let start = road.geometry[0];
                let end = road.geometry[road.geometry.len() - 1];

                let interpolated = start + (end - start) * progress;

                // Обновляем визуальную позицию
                pos.0 = Vec2::new(interpolated.x as f32, interpolated.y as f32);
            }
        }
    }
}