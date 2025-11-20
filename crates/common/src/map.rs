//! Road network graph loading and management.
//!
//! This module handles loading OpenStreetMap data from PBF files and building
//! a routing graph for traffic simulation. It uses OSM highway data to create
//! a directed graph of drivable roads.

use std::collections::HashMap;
use std::fs::File;
use anyhow::{Context, Result};
use osmpbfreader::{OsmObj, OsmPbfReader};
use geo::prelude::*;
use geo::Point;
use glam::DVec2;
use bevy_ecs::prelude::Resource;
use serde::{Serialize, Deserialize};

/// Represents a node in the road network graph.
///
/// Each node corresponds to an intersection or point along a road
/// in the OpenStreetMap data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// OpenStreetMap node ID
    pub id: i64,
    /// Geographic position (longitude, latitude)
    pub pos: DVec2,
}

/// Represents a road segment (edge) in the road network graph.
///
/// Each road connects two nodes and stores information about the
/// segment's length, geometry, and road classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Road {
    /// OpenStreetMap way ID
    pub id: i64,
    /// Starting node ID
    pub start: i64,
    /// Ending node ID
    pub end: i64,
    /// Physical length in meters (calculated using Haversine distance)
    pub length: f64,
    /// Geometric points along the road segment
    pub geometry: Vec<DVec2>,
    /// OSM highway classification (e.g., "motorway", "residential")
    pub highway_type: String,
}

/// The complete road network graph structure.
///
/// Contains all nodes, road segments, and topological information
/// for efficient routing and simulation.
#[derive(Debug, Default, Resource, Serialize, Deserialize)]
pub struct RoadGraph {
    /// All nodes in the network, indexed by OSM node ID
    pub nodes: HashMap<i64, Node>,
    /// All road segments in the network
    pub edges: Vec<Road>,
    /// Adjacency list: maps each node ID to indices of outgoing road segments
    #[serde(skip)]
    pub out_edges: HashMap<i64, Vec<usize>>,
}

impl RoadGraph {
    /// Loads a road network graph from an OpenStreetMap PBF file.
    ///
    /// Parses the OSM data, extracts drivable roads, and builds a routing
    /// graph with nodes and directed edges. Only roads marked as drivable
    /// (motorways, residential streets, etc.) are included.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the .osm.pbf file
    ///
    /// # Returns
    ///
    /// A populated `RoadGraph` ready for routing and simulation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be opened
    /// - The PBF data is malformed
    /// - Required geographic data is missing
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use traffic_common::map::RoadGraph;
    ///
    /// let graph = RoadGraph::load_from_pbf("map.osm.pbf")
    ///     .expect("Failed to load map");
    /// println!("Loaded {} nodes", graph.nodes.len());
    /// ```
    pub fn load_from_pbf(path: &str) -> Result<Self> {
        tracing::info!("🗺️ Loading map from: {}", path);
        let file = File::open(path).context("Could not open map file")?;
        let mut pbf = OsmPbfReader::new(file);

        // Extract nodes and ways that represent highways
        let objs = pbf.get_objs_and_deps(|obj| {
            obj.is_node() || (obj.is_way() && obj.tags().contains_key("highway"))
        })?;

        let mut graph = RoadGraph::default();

        // First pass: collect all nodes
        for obj in objs.values() {
            if let OsmObj::Node(n) = obj {
                graph.nodes.insert(n.id.0, Node {
                    id: n.id.0,
                    pos: DVec2::new(n.lon(), n.lat()),
                });
            }
        }

        // Second pass: process ways to create road segments
        // Each way becomes multiple edge segments for routing
        for obj in objs.values() {
            if let OsmObj::Way(w) = obj {
                let highway = w.tags.get("highway").map(|s| s.as_str()).unwrap_or("");
                if !is_drivable(highway) {
                    continue;
                }

                // Create routing segments between consecutive nodes
                // Each segment preserves the road geometry between two nodes
                for window in w.nodes.windows(2) {
                    let start_id = window[0].0;
                    let end_id = window[1].0;

                    if let (Some(n1), Some(n2)) = (graph.nodes.get(&start_id), graph.nodes.get(&end_id)) {
                        let p1 = Point::new(n1.pos.x, n1.pos.y);
                        let p2 = Point::new(n2.pos.x, n2.pos.y);
                        let dist = p1.haversine_distance(&p2);

                        // Store segment with its endpoints and highway type
                        // Multiple segments from the same way will form curved roads
                        graph.edges.push(Road {
                            id: w.id.0,
                            start: start_id,
                            end: end_id,
                            length: dist,
                            geometry: vec![n1.pos, n2.pos],
                            highway_type: highway.to_string(),
                        });
                    }
                }
            }
        }

        // Build adjacency list for efficient routing
        let mut out_edges: HashMap<i64, Vec<usize>> = HashMap::new();
        for (index, road) in graph.edges.iter().enumerate() {
            out_edges.entry(road.start).or_default().push(index);
        }
        graph.out_edges = out_edges;

        tracing::info!(
            "✅ Map loaded: {} nodes, {} road segments.",
            graph.nodes.len(),
            graph.edges.len()
        );
        Ok(graph)
    }
}

/// Determines if a highway type is suitable for vehicle traffic.
///
/// # Arguments
///
/// * `highway_type` - OSM highway tag value
///
/// # Returns
///
/// `true` if the highway type allows regular vehicle traffic, `false` otherwise.
/// Excludes footpaths, cycleways, and other non-drivable roads.
fn is_drivable(highway_type: &str) -> bool {
    matches!(
        highway_type,
        "motorway" | "trunk" | "primary" | "secondary" | "tertiary" | "residential" | "service" | "living_street"
    )
}