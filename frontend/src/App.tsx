import { useEffect, useState, useRef, useMemo } from 'react';
import MapGL, { NavigationControl } from 'react-map-gl/maplibre';
import DeckGL from '@deck.gl/react';
import { PathLayer, ScatterplotLayer } from '@deck.gl/layers';
import useWebSocket from 'react-use-websocket';
import 'maplibre-gl/dist/maplibre-gl.css';
import './App.css';

// --- ТИПЫ ---
type Coordinate = [number, number];

interface Road {
  id: number;
  geometry: Coordinate[];
}

interface Vehicle {
  id: string;
  lat: number;
  lon: number;
  speed: number;
}

const INITIAL_VIEW_STATE = {
  longitude: 13.4050,
  latitude: 52.5200,
  zoom: 13,
  pitch: 0,
  bearing: 0
};

const COLOR_ROAD = [0, 242, 255];
const COLOR_CAR = [255, 0, 85];

function App() {
  const [roads, setRoads] = useState<Road[]>([]);
  const [vehicles, setVehicles] = useState<Vehicle[]>([]);
  const vehiclesBuffer = useRef<Map<string, Vehicle>>(new Map());

  const { lastMessage } = useWebSocket('ws://localhost:3000/ws', {
    shouldReconnect: () => true,
    onOpen: () => console.log("✅ WebSocket Connected!"),
    onClose: () => console.log("❌ WebSocket Disconnected"),
    onError: (e) => console.error("WebSocket Error:", e),
  });

  // ОБРАБОТКА СООБЩЕНИЙ
  useEffect(() => {
    if (lastMessage !== null) {
      try {
        const rawData = JSON.parse(lastMessage.data);
        
        // ЛОГ ПЕРВОГО СООБЩЕНИЯ (чтобы понять структуру)
        if (vehiclesBuffer.current.size === 0) {
            console.log("📩 First data received:", rawData);
        }

        // Вариант 1: Пришел массив
        if (Array.isArray(rawData)) {
             rawData.forEach(v => vehiclesBuffer.current.set(v.id, v));
        } 
        // Вариант 2: Пришел объект { vehicles: [...] }
        else if (rawData.vehicles && Array.isArray(rawData.vehicles)) {
             rawData.vehicles.forEach((v: Vehicle) => vehiclesBuffer.current.set(v.id, v));
        }
        // Вариант 3: Пришла одна машина { id: ... }
        else if (rawData.id) {
             vehiclesBuffer.current.set(rawData.id, rawData);
        }

      } catch (e) {
        console.error("WS Parse error", e);
      }
    }
  }, [lastMessage]);

  // GAME LOOP
  useEffect(() => {
    const interval = setInterval(() => {
      if (vehiclesBuffer.current.size > 0) {
        setVehicles(Array.from(vehiclesBuffer.current.values()));
      }
    }, 33); 
    return () => clearInterval(interval);
  }, []);

  // ЗАГРУЗКА КАРТЫ
  useEffect(() => {
    fetch('http://localhost:3000/map')
      .then(res => res.json())
      .then((data: Road[]) => {
        console.log(`🗺️ Loaded ${data.length} roads`);
        setRoads(data);
      })
      .catch(console.error);
  }, []);

  const layers = useMemo(() => [
    new PathLayer({
      id: 'road-layer',
      data: roads,
      getPath: (d: Road) => d.geometry,
      getColor: COLOR_ROAD,
      getWidth: 5,
      widthMinPixels: 1, // Чтобы дороги не пропадали при отдалении
      opacity: 0.3
    }),
    
    new ScatterplotLayer({
      id: 'vehicle-layer',
      data: vehicles,
      getPosition: (d: Vehicle) => [d.lon, d.lat],
      getFillColor: COLOR_CAR,
      getRadius: 30,      // [FIX] Увеличили радиус (в метрах)
      radiusMinPixels: 5, // [FIX] Минимальный размер в пикселях (всегда видно)
      opacity: 1,
      stroked: true,
      getLineColor: [255, 255, 255],
      lineWidthMinPixels: 1
    })
  ], [roads, vehicles]);

  return (
    <div className="app-container">
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