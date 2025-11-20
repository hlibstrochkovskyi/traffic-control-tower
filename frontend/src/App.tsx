// frontend/src/App.tsx
import { useEffect, useState, useRef } from 'react' // Добавили useRef
import { MapContainer, TileLayer, CircleMarker, Popup, Polyline } from 'react-leaflet'
import useWebSocket from 'react-use-websocket'
import 'leaflet/dist/leaflet.css'
import './App.css'

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

  // ИСПОЛЬЗУЕМ REF ДЛЯ ХРАНЕНИЯ СОСТОЯНИЯ БЕЗ ПЕРЕРИСОВКИ
  // Это наш буфер. React не перерисовывает компонент, когда меняется ref.
  const vehiclesMap = useRef<Map<string, Vehicle>>(new Map());

  const { lastMessage } = useWebSocket('ws://localhost:3000/ws', {
    shouldReconnect: () => true,
  });

  // 1. Читаем сообщения и обновляем ТОЛЬКО буфер (быстро)
  useEffect(() => {
    if (lastMessage !== null) {
      try {
        const data = JSON.parse(lastMessage.data);
        
        // Проверка: это одиночная машина или список?
        if (data.id && data.lat && data.lon) {
           // Пришла одна машина - обновляем её в карте
           vehiclesMap.current.set(data.id, data);
        } else if (data.vehicles) {
           // (На случай если бэкенд начнет слать пачки)
           data.vehicles.forEach((v: Vehicle) => vehiclesMap.current.set(v.id, v));
        }
      } catch (e) {
        console.error("Parse error", e);
      }
    }
  }, [lastMessage]);

  // 2. Таймер перерисовки (Game Loop для React)
  // Обновляем State (и вызываем рендер) только раз в 50мс (20 FPS)
  useEffect(() => {
    const interval = setInterval(() => {
      if (vehiclesMap.current.size > 0) {
        // Превращаем Map обратно в массив для рендеринга
        setVehicles(Array.from(vehiclesMap.current.values()));
      }
    }, 50); 

    return () => clearInterval(interval);
  }, []);

  // Загрузка карты (осталась без изменений)
  useEffect(() => {
    fetch('http://localhost:3000/map')
      .then(res => res.json())
      .then((data: Road[]) => {
        // Валидация геометрии
        const validRoads = data.map(r => ({
            ...r,
            geometry: r.geometry.map(p => [p[1], p[0]] as Coordinate) // Swap Lat/Lon fix
        }));
        setRoads(validRoads);
        setIsLoadingMap(false);
      })
      .catch(err => {
        console.error(err);
        setMapError("Failed to load map");
        setIsLoadingMap(false);
      });
  }, []);

  return (
    <div className="app-container">
      <div className="sidebar">
        <h2>Traffic Control</h2>
        <div className="stat-box">
          <h3>Active Vehicles</h3>
          <p className="stat-number">{vehicles.length}</p>
        </div>
        <div className="stat-box">
          <h3>Visible Roads</h3>
          <p className="stat-number">{roads.length}</p>
        </div>
      </div>

      <div className="map-container">
        {isLoadingMap ? (
          <div style={{color: 'white', margin: 'auto'}}>Loading Map...</div>
        ) : (
          <MapContainer center={[52.5200, 13.4050]} zoom={13} style={{ height: '100%', width: '100%' }}>
            <TileLayer
              attribution='&copy; OpenStreetMap'
              url="https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png"
            />

            {/* Дороги */}
            {roads.map((road) => (
              <Polyline
                key={road.id}
                positions={road.geometry}
                pathOptions={{ color: '#00f2ff', weight: 1, opacity: 0.3 }}
              />
            ))}

            {/* Машины (рендерим только первые 1000 чтобы не висело) */}
            {vehicles.slice(0, 1000).map((v) => (
              <CircleMarker 
                key={v.id} 
                center={[v.lat, v.lon]} 
                radius={3}
                pathOptions={{ color: '#ff0055', fillColor: '#ff0055', fillOpacity: 1, stroke: false }}
              />
            ))}
          </MapContainer>
        )}
      </div>
    </div>
  )
}

export default App