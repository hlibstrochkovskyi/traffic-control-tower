/**
 * Traffic Control Tower - Main Application Component
 * 
 * Real-time traffic visualization dashboard displaying vehicle movements
 * on a road network using WebGL-accelerated rendering via Deck.gl.
 * 
 * Features:
 * - Live vehicle tracking via WebSocket connection
 * - Road network visualization from OpenStreetMap data
 * - Interactive map controls with zoom and pan
 * - Real-time statistics sidebar
 * 
 * @module App
 */

import { useEffect, useState, useRef, useMemo } from 'react';
import MapGL, { NavigationControl } from 'react-map-gl/maplibre';
import DeckGL from '@deck.gl/react';
import { PathLayer, ScatterplotLayer } from '@deck.gl/layers';
import useWebSocket from 'react-use-websocket';
import 'maplibre-gl/dist/maplibre-gl.css';
import './App.css';

// --- TYPE DEFINITIONS ---

/** Geographic coordinate tuple [longitude, latitude] */
type Coordinate = [number, number];

/**
 * Road segment with unique identifier and geometry path.
 */
interface Road {
  /** Unique road identifier from OpenStreetMap */
  id: number;
  /** Sequence of coordinates defining the road path */
  geometry: Coordinate[];
}

/**
 * Vehicle telemetry data received from the simulation.
 */
interface Vehicle {
  /** Unique vehicle identifier (e.g., "car_42") */
  id: string;
  /** Latitude position in decimal degrees */
  lat: number;
  /** Longitude position in decimal degrees */
  lon: number;
  /** Current speed in meters per second */
  speed: number;
}

// --- CONSTANTS ---

/** Initial map viewport centered on Berlin */
const INITIAL_VIEW_STATE = {
  longitude: 13.4050,
  latitude: 52.5200,
  zoom: 13,
  pitch: 0,
  bearing: 0
};

/** Cyan color for road visualization [R, G, B] */
const COLOR_ROAD = [0, 242, 255];

/** Hot pink color for vehicle markers [R, G, B] */
const COLOR_CAR = [255, 0, 85];

/**
 * Main application component managing map visualization and real-time data.
 * 
 * Architecture:
 * - Uses a buffered update pattern to prevent excessive re-renders
 * - WebSocket messages update an in-memory buffer
 * - Game loop syncs buffer to React state at 30 FPS
 * - Memoized layers prevent unnecessary Deck.gl recalculations
 * 
 * @returns React component rendering the traffic control dashboard
 */
function App() {
  /** Road network segments loaded from the API */
  const [roads, setRoads] = useState<Road[]>([]);
  
  /** Currently tracked vehicles for rendering */
  const [vehicles, setVehicles] = useState<Vehicle[]>([]);
  
  /** 
   * In-memory buffer for incoming vehicle updates.
   * Prevents React state updates on every WebSocket message.
   */
  const vehiclesBuffer = useRef<Map<string, Vehicle>>(new Map());

  // --- WEBSOCKET CONNECTION ---
  
  const { lastMessage } = useWebSocket('ws://localhost:3000/ws', {
    shouldReconnect: () => true,
    onOpen: () => console.log("✅ WebSocket Connected!"),
    onClose: () => console.log("❌ WebSocket Disconnected"),
    onError: (e) => console.error("WebSocket Error:", e),
  });

  /**
   * Processes incoming WebSocket messages and updates the vehicle buffer.
   * 
   * Handles three message formats:
   * 1. Array of vehicles: `[{id, lat, lon, speed}, ...]`
   * 2. Wrapped array: `{vehicles: [{...}, ...]}`
   * 3. Single vehicle: `{id, lat, lon, speed}`
   * 
   * The buffer uses vehicle ID as key to automatically handle updates.
   */
  useEffect(() => {
    if (lastMessage !== null) {
      try {
        const rawData = JSON.parse(lastMessage.data);
        
        // Log first message for debugging structure
        if (vehiclesBuffer.current.size === 0) {
            console.log("📩 First data received:", rawData);
        }

        // Case 1: Array of vehicles
        if (Array.isArray(rawData)) {
             rawData.forEach(v => vehiclesBuffer.current.set(v.id, v));
        } 
        // Case 2: Wrapped in vehicles property
        else if (rawData.vehicles && Array.isArray(rawData.vehicles)) {
             rawData.vehicles.forEach((v: Vehicle) => vehiclesBuffer.current.set(v.id, v));
        }
        // Case 3: Single vehicle object
        else if (rawData.id) {
             vehiclesBuffer.current.set(rawData.id, rawData);
        }

      } catch (e) {
        console.error("WS Parse error", e);
      }
    }
  }, [lastMessage]);

  /**
   * Game loop synchronizing the vehicle buffer to React state.
   * 
   * Runs at ~30 FPS (every 33ms) to balance smoothness with performance.
   * Only triggers re-render if buffer contains data.
   */
  useEffect(() => {
    const interval = setInterval(() => {
      if (vehiclesBuffer.current.size > 0) {
        setVehicles(Array.from(vehiclesBuffer.current.values()));
      }
    }, 33); 
    return () => clearInterval(interval);
  }, []);

  /**
   * Loads the road network from the API on component mount.
   * 
   * Fetches all road segments that will be displayed on the map.
   * Roads are static and loaded once during initialization.
   */
  useEffect(() => {
    fetch('http://localhost:3000/map')
      .then(res => res.json())
      .then((data: Road[]) => {
        console.log(`🗺️ Loaded ${data.length} roads`);
        setRoads(data);
      })
      .catch(console.error);
  }, []);

  /**
   * Memoized Deck.gl layers for efficient rendering.
   * 
   * Layers are only recreated when roads or vehicles data changes,
   * preventing unnecessary WebGL buffer updates.
   */
  const layers = useMemo(() => [
    // Road network layer
    new PathLayer({
      id: 'road-layer',
      data: roads,
      getPath: (d: Road) => d.geometry,
      getColor: COLOR_ROAD,
      getWidth: 5,
      widthMinPixels: 1, // Ensure roads remain visible when zoomed out
      opacity: 0.3
    }),
    
    // Vehicle markers layer
    new ScatterplotLayer({
      id: 'vehicle-layer',
      data: vehicles,
      getPosition: (d: Vehicle) => [d.lon, d.lat],
      getFillColor: COLOR_CAR,
      getRadius: 30,      // Radius in meters at zoom level 1
      radiusMinPixels: 5, // Minimum pixel size (always visible)
      opacity: 1,
      stroked: true,
      getLineColor: [255, 255, 255],
      lineWidthMinPixels: 1
    })
  ], [roads, vehicles]);

  return (
    <div className="app-container">
      {/* Statistics Sidebar */}
      <div className="sidebar">
        <h2>Traffic Control</h2>
        <div className="stat-box">
          <h3>Active Vehicles</h3>
          <p className="stat-number" style={{color: '#ff0055'}}>{vehicles.length}</p>
        </div>
        <div className="stat-box">
          <h3>Visible Roads</h3>
          <p className="stat-number">{roads.length}</p>
        </div>
      </div>

      {/* Interactive Map */}
      <div className="map-container">
        <DeckGL
          initialViewState={INITIAL_VIEW_STATE}
          controller={true}
          layers={layers}
          getTooltip={({object}: any) => object && object.id ? `${object.id}` : null}
        >
          <MapGL
            mapStyle="https://basemaps.cartocdn.com/gl/dark-matter-gl-style/style.json"
            reuseMaps
          >
            <NavigationControl position="top-left" />
          </MapGL>
        </DeckGL>
      </div>
    </div>
  );
}

export default App;