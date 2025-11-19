use std::collections::HashMap;
use std::fs::File;
use anyhow::{Context, Result};
use osmpbfreader::{OsmObj, OsmPbfReader};
use geo::prelude::*; // Для Haversine distance
use geo::Point;
use glam::DVec2; // Используем Double precision для координат
use bevy_ecs::prelude::Resource; // <--- ВАЖНО: Импорт для ECS

// Узел графа (перекресток)
#[derive(Debug, Clone)]
pub struct Node {
    pub id: i64,
    pub pos: DVec2, // x=lon, y=lat
}

// Ребро графа (участок дороги)
#[derive(Debug, Clone)]
pub struct Road {
    pub id: i64,          // ID пути из OSM
    pub start: i64,       // ID начального Node
    pub end: i64,         // ID конечного Node
    pub length: f64,      // Длина в метрах
    pub geometry: Vec<DVec2>, // Точки формы дороги
}

#[derive(Debug, Default, Resource)] // <--- Resource позволяет хранить карту в World
pub struct RoadGraph {
    pub nodes: HashMap<i64, Node>,
    pub edges: Vec<Road>,
    // Индекс: ID Узла -> Список индексов исходящих дорог в массиве edges
    pub out_edges: HashMap<i64, Vec<usize>>,
}

impl RoadGraph {
    pub fn load_from_pbf(path: &str) -> Result<Self> {
        tracing::info!("🗺️ Loading map from: {}", path);

        let file = File::open(path).context("Could not open map file")?;
        let mut pbf = OsmPbfReader::new(file);

        // 1. Читаем всё и фильтруем только нужное
        let objs = pbf.get_objs_and_deps(|obj| {
            obj.is_node() || (obj.is_way() && obj.tags().contains_key("highway"))
        })?;

        let mut graph = RoadGraph::default();

        // 2. Сначала собираем все Nodes
        for (_id, obj) in &objs {
            if let OsmObj::Node(n) = obj {
                // Берем ID из самого объекта n.id.0
                graph.nodes.insert(n.id.0, Node {
                    id: n.id.0,
                    pos: DVec2::new(n.lon(), n.lat()),
                });
            }
        }

        // 3. Теперь собираем Дороги (Ways)
        for (_id, obj) in &objs {
            if let OsmObj::Way(w) = obj {
                let highway = w.tags.get("highway").map(|s| s.as_str()).unwrap_or("");
                if !is_drivable(highway) {
                    continue;
                }

                for window in w.nodes.windows(2) {
                    let start_id = window[0].0;
                    let end_id = window[1].0;

                    if let (Some(n1), Some(n2)) = (graph.nodes.get(&start_id), graph.nodes.get(&end_id)) {
                        let p1 = Point::new(n1.pos.x, n1.pos.y);
                        let p2 = Point::new(n2.pos.x, n2.pos.y);
                        let dist = p1.haversine_distance(&p2);

                        graph.edges.push(Road {
                            id: w.id.0, // Берем ID из w.id.0
                            start: start_id,
                            end: end_id,
                            length: dist,
                            geometry: vec![n1.pos, n2.pos],
                        });
                    }
                }
            }
        }

        // 4. Строим топологию (индекс связности)
        tracing::info!("🚧 Building graph topology...");
        let mut out_edges: HashMap<i64, Vec<usize>> = HashMap::new();

        for (index, road) in graph.edges.iter().enumerate() {
            out_edges.entry(road.start).or_default().push(index);
        }
        graph.out_edges = out_edges;

        tracing::info!("✅ Map loaded: {} nodes, {} road segments. Topology built.",
            graph.nodes.len(),
            graph.edges.len()
        );
        Ok(graph)
    }
}

fn is_drivable(highway_type: &str) -> bool {
    match highway_type {
        "motorway" | "trunk" | "primary" | "secondary" | "tertiary" | "residential" | "service" | "living_street" => true,
        _ => false,
    }
}