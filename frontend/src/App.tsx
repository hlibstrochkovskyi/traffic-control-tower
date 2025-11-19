import { useState, useEffect, useRef } from 'react';
import DeckGL from '@deck.gl/react';
import { ScatterplotLayer } from '@deck.gl/layers';
import { Map } from 'react-map-gl';
import 'mapbox-gl/dist/mapbox-gl.css';

// Description of the vehicle type
interface Vehicle {
  id: string;
  lat: number;
  lon: number;
  speed: number;
}

// Mapbox token (you can keep a placeholder if you don't have your own)
const MAPBOX_TOKEN = "pk.eyJ1IjoidHJhZmZpYy10b3dlciIsImEiOiJjbHUxb3BqbW8wMTZ4MmtyemE2ZHp6enJ6In0.placeholder"; 

export default function App() {
  const [vehicles, setVehicles] = useState<Vehicle[]>([]);
  const [viewState, setViewState] = useState({
    longitude: 13.40,
    latitude: 52.52,
    zoom: 11,
    pitch: 0,
    bearing: 0
  });

  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    // Connect to the WebSocket API
    const ws = new WebSocket(
      `ws://localhost:3000/ws?lat=${viewState.latitude}&lon=${viewState.longitude}&radius_km=20`
    );

    ws.onopen = () => console.log('✅ WebSocket connected');
    
    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        console.log("Пришли данные:", data);
        setVehicles(data);
      } catch (err) {
        console.error('Parse error:', err);
      }
    };

    ws.onerror = (error) => console.error('WebSocket error:', error);
    ws.onclose = () => console.log('❌ WebSocket closed');

    wsRef.current = ws;

    return () => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.close();
      }
    };
  }, []);

  // Vehicles layer
  const layer = new ScatterplotLayer({
    id: 'vehicles',
    data: vehicles,
    pickable: true,
    opacity: 0.8,
    stroked: true,
    filled: true,
    radiusScale: 6,
    radiusMinPixels: 3,
    radiusMaxPixels: 15,
    lineWidthMinPixels: 1,
    getPosition: (d: Vehicle) => [d.lon, d.lat],
    getFillColor: (d: Vehicle) => {
      const speed = d.speed; 
      if (speed < 10) return [255, 0, 0];       // Red
      if (speed < 30) return [255, 165, 0];     // Orange
      return [0, 255, 0];                       // Green
    },
    getLineColor: [0, 0, 0],
    onClick: (info: any) => {
       if (info.object) {
         alert(`🚗 Vehicle: ${info.object.id}\nSpeed: ${info.object.speed.toFixed(1)} km/h`);
       }
    }
  });

  return (
    <div style={{ width: '100vw', height: '100vh', position: 'relative', background: '#111' }}>
      <DeckGL
        viewState={viewState}
        controller={true}
        layers={[layer]}
        onViewStateChange={(e: any) => setViewState(e.viewState)}
      >
        <Map
          mapboxAccessToken={MAPBOX_TOKEN}
          mapStyle="mapbox://styles/mapbox/dark-v9"
        />
      </DeckGL>

      {/* Stats panel */}
      <div style={{
        position: 'absolute',
        top: 20,
        left: 20,
        zIndex: 1,
        background: 'rgba(30,30,30,0.9)',
        color: 'white',
        padding: '20px',
        borderRadius: '12px',
        fontFamily: 'monospace',
        fontSize: '14px',
        border: '1px solid #444',
        pointerEvents: 'none'
      }}>
        <div style={{ fontSize: '18px', marginBottom: '10px', fontWeight: 'bold' }}>
          🚦 Traffic Tower
        </div>
        <div style={{ color: '#4ade80', fontSize: '1.2em' }}>
          Active Vehicles: {vehicles.length}
        </div>
        <div style={{ color: '#aaa', marginTop: '5px', fontSize: '12px' }}>
          Live Feed from Redis
        </div>
      </div>
    </div>
  );
}