use glam::Vec2;

/// Returns a Berlin ring route approximating the S-Bahn Ringbahn.
/// This creates a recognizable circular pattern around central Berlin
/// with approximately 15 waypoints.
pub fn berlin_ring_route() -> Vec<Vec2> {
    vec![
        // Starting from Westkreuz (West)
        Vec2::new(13.3884, 52.5244),
        // Moving south towards Südkreuz area
        Vec2::new(13.3650, 52.5050),
        Vec2::new(13.3650, 52.4750),
        // Moving east towards Tempelhof
        Vec2::new(13.3900, 52.4600),
        // Moving towards Ostkreuz area
        Vec2::new(13.4350, 52.4650),
        Vec2::new(13.5030, 52.4650),
        // Moving north-east
        Vec2::new(13.5200, 52.4900),
        // Continuing north towards Ostkreuz
        Vec2::new(13.5200, 52.5150),
        // Moving towards Prenzlauer Berg
        Vec2::new(13.5050, 52.5400),
        // Moving north-west towards Gesundbrunnen
        Vec2::new(13.4800, 52.5550),
        // Moving towards Wedding
        Vec2::new(13.4500, 52.5650),
        // Moving west towards Westhafen
        Vec2::new(13.4200, 52.5650),
        // Moving towards Charlottenburg
        Vec2::new(13.3900, 52.5550),
        // Moving south-west back towards Westkreuz
        Vec2::new(13.3750, 52.5400),
        Vec2::new(13.3750, 52.5300),
    ]
}
