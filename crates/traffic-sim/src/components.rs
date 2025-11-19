use bevy_ecs::prelude::*;
use glam::Vec2;

// Уникальный ID машины (например, "car_42")
#[derive(Component, Debug, Clone)]
pub struct VehicleId(pub String);

// Визуальная позиция (x=longitude, y=latitude)
// В "рельсовой" системе это вычисляемое поле: мы берем GraphPosition и считаем координаты.
#[derive(Component, Debug, Clone, Copy)]
pub struct Position(pub Vec2);

// Вектор скорости (для визуализации и broadcast)
#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity(pub Vec2);

// --- НОВЫЙ КОМПОНЕНТ: Логическая позиция на графе ---
// Машина знает только: "Я на дороге №500, проехала 30 метров от начала"
#[derive(Component, Debug, Clone)]
pub struct GraphPosition {
    pub edge_index: usize, // Индекс дороги в массиве RoadGraph.edges
    pub distance: f64,     // Пройденное расстояние в метрах по этой дороге
}

// Желаемая скорость (м/с)
#[derive(Component, Debug, Clone, Copy)]
pub struct TargetSpeed(pub f32);