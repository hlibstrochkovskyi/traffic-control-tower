import { useEffect, useState } from 'react'
import { MapContainer, TileLayer, CircleMarker, Popup, Polyline } from 'react-leaflet'
import useWebSocket from 'react-use-websocket'
import 'leaflet/dist/leaflet.css'
import './App.css'

// Типы данных (совпадают с Rust)
interface DVec2 {
  x: number; // lon
  y: number; // lat
}

interface Road {
  id: number;
  geometry: DVec2[]; // Массив точек
}

interface Vehicle {
  id: string;
  lat: number;
  lon: number;
  speed: number;
}

function App() {
  const [vehicles, setVehicles] = useState<Vehicle[]>([]);
  const [roads, setRoads] = useState<Road[]>([]);
  const [isLoadingMap, setIsLoadingMap] = useState(true);

  // 1. WebSocket для машин
  const { lastMessage } = useWebSocket('ws://localhost:3000/ws', {
    shouldReconnect: () => true,
  });

  useEffect(() => {
    if (lastMessage !== null) {
      try {
        const data = JSON.parse(lastMessage.data);
        if (data.vehicles) {
          setVehicles(data.vehicles);
        }
      } catch (e) {
        console.error("Parse error", e);
      }
    }
  }, [lastMessage]);

  // 2. Загрузка карты при старте
  useEffect(() => {
    fetch('http://localhost:3000/map')
      .then(res => res.json())
      .then((data: Road[]) => {
        console.log(`Received ${data.length} roads from API`);
        
        // --- ВАЖНОЕ ИСПРАВЛЕНИЕ ---
        // Берлин огромный (600k дорог). Браузер умрет, если рисовать всё.
        // Берем только первые 3000 дорог для теста визуализации.
        const safeSubset = data.slice(0, 3000); 
        
        setRoads(safeSubset);
        setIsLoadingMap(false);
      })
      .catch(err => {
        console.error("Failed to load map", err);
        setIsLoadingMap(false);
      });
  }, []);

  return (
    <div className="app-container">
      <div className="sidebar">
        <h2>Traffic Control Tower</h2>
        <div className="stat-box">
          <h3>Active Vehicles</h3>
          <p className="stat-number">{vehicles.length}</p>
        </div>
        <div className="stat-box">
          <h3>Visible Roads</h3>
          <p className="stat-number">
            {isLoadingMap ? "Loading..." : roads.length}
          </p>
        </div>
        <div className="vehicle-list">
          {vehicles.slice(0, 10).map(v => (
            <div key={v.id} className="vehicle-item">
              🚗 {v.id} <span className="speed">{v.speed.toFixed(1)} km/h</span>
            </div>
          ))}
        </div>
      </div>

      <div className="map-container">
        {/* Центр Берлина (Alexanderplatz) */}
        <MapContainer center={[52.5200, 13.4050]} zoom={14} style={{ height: '100%', width: '100%' }}>
          
          {/* Темная тема карты (Cyberpunk style) */}
          <TileLayer
            attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OSM</a>'
            url="https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png"
          />

          {/* ОТРИСОВКА ДОРОГ (Линии) */}
          {roads.map((road) => (
            <Polyline
              key={road.id}
              // Leaflet ждет [lat, lon], а у нас [x=lon, y=lat]. Меняем местами!
              positions={road.geometry.map(p => [p.y, p.x])}
              pathOptions={{ color: '#00f2ff', weight: 2, opacity: 0.5 }}
            />
          ))}

          {/* ОТРИСОВКА МАШИН (Точки) */}
          {/* Ограничиваем отрисовку 500 машинами, чтобы не лагало */}
          {vehicles.slice(0, 500).map((v) => (
            <CircleMarker 
              key={v.id} 
              center={[v.lat, v.lon]} 
              radius={4}
              pathOptions={{ color: '#ff0055', fillColor: '#ff0055', fillOpacity: 1 }}
            >
              <Popup>
                <b>{v.id}</b><br/>Speed: {v.speed.toFixed(1)}
              </Popup>
            </CircleMarker>
          ))}
        </MapContainer>
      </div>
    </div>
  )
}

export default App