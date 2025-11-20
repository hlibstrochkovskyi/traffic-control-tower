use std::collections::HashMap;
use std::fs::File;
use anyhow::{Context, Result};
use osmpbfreader::{OsmObj, OsmPbfReader};
use geo::prelude::*;
use geo::Point;
use glam::DVec2;
use bevy_ecs::prelude::Resource;
use serde::{Serialize, Deserialize}; // <--- Добавили для JSON

// Узел графа
#[derive(Debug, Clone, Serialize, Deserialize)] // <--- Serialize
pub struct Node {
    pub id: i64,
    pub pos: DVec2,
}

// Ребро графа (участок дороги)
#[derive(Debug, Clone, Serialize, Deserialize)] // <--- Serialize
pub struct Road {
    pub id: i64,
    pub start: i64,
    pub end: i64,
    pub length: f64,
    pub geometry: Vec<DVec2>,
}

// Добавляем Serialize и Deserialize в список
#[derive(Debug, Default, Resource, Serialize, Deserialize)]
pub struct RoadGraph {
    pub nodes: HashMap<i64, Node>,
    pub edges: Vec<Road>,
    // Теперь #[serde(skip)] сработает, так как структура сериализуемая
    #[serde(skip)]
    pub out_edges: HashMap<i64, Vec<usize>>,
}

impl RoadGraph {
    pub fn load_from_pbf(path: &str) -> Result<Self> {
        // ... КОД ОСТАЕТСЯ ТОТ ЖЕ, ЧТО БЫЛ РАНЬШЕ ...
        // Скопируй сюда тело функции load_from_pbf и is_drivable из старого map.rs
        // (или скажи, если нужно прислать полный код, чтобы не искать)

        tracing::info!("🗺️ Loading map from: {}", path);
        let file = File::open(path).context("Could not open map file")?;
        let mut pbf = OsmPbfReader::new(file);

        let objs = pbf.get_objs_and_deps(|obj| {
            obj.is_node() || (obj.is_way() && obj.tags().contains_key("highway"))
        })?;

        let mut graph = RoadGraph::default();

        for (_id, obj) in &objs {
            if let OsmObj::Node(n) = obj {
                graph.nodes.insert(n.id.0, Node {
                    id: n.id.0,
                    pos: DVec2::new(n.lon(), n.lat()),
                });
            }
        }

        // Process ways to create road segments
        // Each way becomes multiple edge segments for routing,
        // but we preserve the full geometry for visualization
        for (_id, obj) in &objs {
            if let OsmObj::Way(w) = obj {
                let highway = w.tags.get("highway").map(|s| s.as_str()).unwrap_or("");
                if !is_drivable(highway) {
                    continue;
                }

                // Collect all points in this way for full geometry
                let way_geometry: Vec<DVec2> = w.nodes
                    .iter()
                    .filter_map(|node_id| {
                        graph.nodes.get(&node_id.0).map(|n| n.pos)
                    })
                    .collect();

                if way_geometry.len() < 2 {
                    continue;
                }

                // Create routing segments between consecutive nodes
                // Each segment stores the full geometry of its portion of the way
                for window in w.nodes.windows(2) {
                    let start_id = window[0].0;
                    let end_id = window[1].0;

                    if let (Some(n1), Some(n2)) = (graph.nodes.get(&start_id), graph.nodes.get(&end_id)) {
                        let p1 = Point::new(n1.pos.x, n1.pos.y);
                        let p2 = Point::new(n2.pos.x, n2.pos.y);
                        let dist = p1.haversine_distance(&p2);

                        // For each routing segment, store just its two endpoints
                        // This keeps routing simple while the full way geometry
                        // is available for visualization via the way_id
                        graph.edges.push(Road {
                            id: w.id.0,
                            start: start_id,
                            end: end_id,
                            length: dist,
                            geometry: vec![n1.pos, n2.pos], // Just segment endpoints
                        });
                    }
                }
            }
        }

        // Топология
        let mut out_edges: HashMap<i64, Vec<usize>> = HashMap::new();
        for (index, road) in graph.edges.iter().enumerate() {
            out_edges.entry(road.start).or_default().push(index);
        }
        graph.out_edges = out_edges;

        tracing::info!("✅ Map loaded: {} nodes, {} road segments.", graph.nodes.len(), graph.edges.len());
        Ok(graph)
    }
}

fn is_drivable(highway_type: &str) -> bool {
    match highway_type {
        "motorway" | "trunk" | "primary" | "secondary" | "tertiary" | "residential" | "service" | "living_street" => true,
        _ => false,
    }
}