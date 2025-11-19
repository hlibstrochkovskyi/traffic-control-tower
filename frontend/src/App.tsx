import { useState, useEffect, useRef } from 'react';
import DeckGL from '@deck.gl/react';
import { ScatterplotLayer } from '@deck.gl/layers';
import { Map } from 'react-map-gl';
import maplibregl from 'maplibre-gl';
import 'maplibre-gl/dist/maplibre-gl.css';

// Используем светлую карту (Positron), чтобы видеть улицы
const MAP_STYLE = "https://basemaps.cartocdn.com/gl/positron-gl-style/style.json";

interface Vehicle {
  id: string;
  lat: number;
  lon: number;
  speed: number;
}

export default function App() {
  const [vehicles, setVehicles] = useState<Vehicle[]>([]);
  
  // Начальная позиция камеры - Центр Берлина
  const [viewState, setViewState] = useState({
    longitude: 13.40,
    latitude: 52.52,
    zoom: 10.5, // Чуть отдалим, чтобы видеть все кольцо
    pitch: 0,
    bearing: 0
  });

  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    // Подключаемся к WebSocket
    const ws = new WebSocket(
      `ws://localhost:3000/ws?lat=${viewState.latitude}&lon=${viewState.longitude}&radius_km=50`
    );

    ws.onopen = () => console.log('✅ WebSocket connected');
    
    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        // Если данных нет, не обновляем стейт пустым массивом, чтобы не моргало
        if (data && data.length > 0) {
            setVehicles(data);
        }
      } catch (err) {
        console.error('Parse error:', err);
      }
    };

    wsRef.current = ws;

    return () => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.close();
      }
    };
  }, []); // Запускаем один раз при старте

const layer = new ScatterplotLayer({
    id: 'vehicles',
    data: vehicles,
    pickable: true,
    opacity: 1,             // Полная непрозрачность
    stroked: true,
    filled: true,
    radiusScale: 1,         // Масштаб 1:1 к метрам (примерно)
    radiusMinPixels: 8,     // ОЧЕНЬ КРУПНЫЕ ТОЧКИ (чтобы точно увидеть)
    radiusMaxPixels: 20,
    getPosition: (d: Vehicle) => [d.lon, d.lat],
    getFillColor: (d: Vehicle) => {
      // Логика цвета от скорости (которую мы задали в Rust)
      // 0.0008 (Rust) ~ 80 (в единицах фронта после умножения)
      // 0.0003 (Rust) ~ 30
      
      if (d.speed > 50) { 
          return [255, 0, 0]; // КРАСНЫЙ (Линия)
      } else {
          return [0, 100, 255]; // СИНИЙ (Кольцо)
      }
    },
    getLineColor: [255, 255, 255], // Белая обводка для контраста
    lineWidthMinPixels: 2,
    updateTriggers: {
        getFillColor: [vehicles]
    }
  });

  return (
    <div style={{ width: '100vw', height: '100vh', position: 'relative', background: '#e5e5e5' }}>
      <DeckGL
        initialViewState={viewState}
        controller={true}
        layers={[layer]}
        onViewStateChange={(e: any) => setViewState(e.viewState)}
      >
        <Map
          mapLib={maplibregl}
          mapStyle={MAP_STYLE}
        />
      </DeckGL>
      
      {/* Панель статистики */}
      <div style={{
        position: 'absolute',
        top: 20,
        left: 20,
        zIndex: 1,
        background: 'white',
        padding: '20px',
        borderRadius: '8px',
        boxShadow: '0 4px 6px rgba(0,0,0,0.1)',
        fontFamily: 'sans-serif',
        fontSize: '14px',
      }}>
        <div style={{ fontSize: '18px', marginBottom: '10px', fontWeight: 'bold', color: '#333' }}>
          🚦 Berlin Traffic Tower
        </div>
        <div style={{ color: '#2563eb', fontSize: '1.2em', fontWeight: 'bold' }}>
          Vehicles: {vehicles.length}
        </div>
        <div style={{ color: '#666', marginTop: '5px', fontSize: '12px' }}>
          Real-time Simulation
        </div>
      </div>
    </div>
  );
}