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
  const [mapError, setMapError] = useState<string | null>(null);

  // Подключение к WebSocket (порт 3000, как в твоем docker-compose/api)
  const { lastMessage } = useWebSocket('ws://localhost:3000/ws', {
    shouldReconnect: () => true,
    onError: (event) => {
      console.error('WebSocket error:', event);
    },
    onOpen: () => {
      console.log('WebSocket connected');
    },
    onClose: () => {
      console.log('WebSocket disconnected');
    }
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
    console.log('🗺️ Loading road map...');
    fetch('http://localhost:3000/map')
      .then(res => {
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}: ${res.statusText}`);
        }
        return res.json();
      })
      .then((data: Road[]) => {
        console.log(`✅ Map data loaded successfully: ${data.length} road segments`);
        
        // Log some statistics about the roads
        const totalPoints = data.reduce((sum, road) => sum + road.geometry.length, 0);
        console.log(`📊 Total geometry points: ${totalPoints}`);
        console.log(`📊 Average points per road: ${(totalPoints / data.length).toFixed(2)}`);
        
        // Show sample of first road for debugging
        if (data.length > 0) {
          console.log('📍 Sample road:', {
            id: data[0].id,
            points: data[0].geometry.length,
            firstPoint: data[0].geometry[0],
            lastPoint: data[0].geometry[data[0].geometry.length - 1]
          });
        }
        
        setRoads(data);
        setIsLoadingMap(false);
        setMapError(null);
      })
      .catch(err => {
        console.error("❌ Failed to load map:", err);
        setMapError(err.message || 'Unknown error');
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
            {isLoadingMap ? "Loading..." : mapError ? "Error!" : roads.length}
          </p>
          {mapError && (
            <p style={{ color: '#ff4444', fontSize: '12px', marginTop: '8px' }}>
              {mapError}
            </p>
          )}
        </div>
      </div>

      <div className="map-container">
        {/* Центр карты (Берлин) */}
        {isLoadingMap ? (
          <div style={{ 
            display: 'flex', 
            justifyContent: 'center', 
            alignItems: 'center', 
            height: '100%',
            color: 'white',
            fontSize: '24px'
          }}>
            🗺️ Loading map...
          </div>
        ) : mapError ? (
          <div style={{ 
            display: 'flex', 
            flexDirection: 'column',
            justifyContent: 'center', 
            alignItems: 'center', 
            height: '100%',
            color: '#ff4444',
            fontSize: '18px',
            padding: '20px'
          }}>
            <div>❌ Map loading failed</div>
            <div style={{ fontSize: '14px', marginTop: '10px' }}>{mapError}</div>
          </div>
        ) : (
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
        )}
      </div>
    </div>
  )
}

export default App