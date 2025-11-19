use bevy_ecs::prelude::*;
use glam::Vec2;

// --- РЕСУРСЫ (Глобальные данные) ---

// Хранит время последнего кадра
#[derive(Resource, Debug, Clone, Copy)]
pub struct DeltaTime(pub f32);

// --- КОМПОНЕНТЫ (Данные машин) ---

// Уникальный ID машины (например, "car_42")
#[derive(Component, Debug, Clone)]
pub struct VehicleId(pub String);

// Визуальная позиция (x=longitude, y=latitude)
#[derive(Component, Debug, Clone, Copy)]
pub struct Position(pub Vec2);

// Вектор скорости
#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity(pub Vec2);

// Логическая позиция на графе
#[derive(Component, Debug, Clone)]
pub struct GraphPosition {
    pub edge_index: usize, // Индекс дороги
    pub distance: f64,     // Прогресс по дороге
}

// Желаемая скорость
#[derive(Component, Debug, Clone, Copy)]
pub struct TargetSpeed(pub f32);