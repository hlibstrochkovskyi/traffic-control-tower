import { useEffect, useState } from 'react'
import { MapContainer, TileLayer, CircleMarker, Popup, Polyline } from 'react-leaflet'
import useWebSocket from 'react-use-websocket'
import 'leaflet/dist/leaflet.css'
import './App.css'

// 1. Исправляем тип координат. Rust (glam) шлет массив [lon, lat]
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

function App() {
  const [vehicles, setVehicles] = useState<Vehicle[]>([]);
  const [roads, setRoads] = useState<Road[]>([]);
  const [isLoadingMap, setIsLoadingMap] = useState(true);

  // Подключение к WebSocket (порт 3000, как в твоем docker-compose/api)
  const { lastMessage } = useWebSocket('ws://localhost:3000/ws', {
    shouldReconnect: () => true,
  });

  // Обработка сообщений от машин
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

  // Загрузка карты дорог (один раз при старте)
  useEffect(() => {
    fetch('http://localhost:3000/map')
      .then(res => res.json())
      .then((data: Road[]) => {
        console.log("Map data loaded, segments:", data.length);
        // Берем первые 3000 кусков дорог, чтобы не положить браузер
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
      </div>

      <div className="map-container">
        {/* Центр карты (Берлин) */}
        <MapContainer center={[52.5200, 13.4050]} zoom={14} style={{ height: '100%', width: '100%' }}>
          <TileLayer
            attribution='OSM'
            url="https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png"
          />

          {/* ОТРИСОВКА ДОРОГ (Синие линии) */}
          {roads.map((road) => (
            <Polyline
              key={road.id}
              // ВАЖНО: Leaflet ждет [Lat, Lon], а GeoJSON/Rust дает [Lon, Lat].
              // Поэтому меняем p[1] и p[0] местами.
              positions={road.geometry.map(p => [p[1], p[0]])}
              pathOptions={{ color: '#00f2ff', weight: 2, opacity: 0.6 }}
            />
          ))}

          {/* ОТРИСОВКА МАШИН (Красные точки) */}
          {vehicles.slice(0, 500).map((v) => (
            <CircleMarker 
              key={v.id} 
              center={[v.lat, v.lon]} 
              radius={4}
              pathOptions={{ color: '#ff0055', fillColor: '#ff0055', fillOpacity: 1 }}
            >
               <Popup>{v.id}</Popup>
            </CircleMarker>
          ))}
        </MapContainer>
      </div>
    </div>
  )
}

export default App