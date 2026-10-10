"use client";
import { useEffect, useRef, useState } from "react";
import * as L from "leaflet";
import "leaflet/dist/leaflet.css";
import type { StudentRoute } from "@/lib/api";

export default function StudentRouteMap({ legs }: { legs: StudentRoute["features"] }) {
  const container = useRef<HTMLDivElement>(null);
  const map = useRef<L.Map | null>(null);
  const route = useRef<L.FeatureGroup | null>(null);
  const [tileError, setTileError] = useState(false);

  useEffect(() => {
    if (!container.current) return;
    const instance = L.map(container.current, { scrollWheelZoom: false, zoomControl: false });
    map.current = instance;
    route.current = L.featureGroup().addTo(instance);
    L.control.zoom({ zoomInTitle: "Přiblížit trasu", zoomOutTitle: "Oddálit trasu" }).addTo(instance);
    L.tileLayer("https://tile.openstreetmap.org/{z}/{x}/{y}.png", {
      maxZoom: 19, keepBuffer: 0,
      attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
    }).on("tileerror", () => setTileError(true)).addTo(instance);
    const observer = new ResizeObserver(() => {
      instance.invalidateSize({ pan: false });
      const bounds = route.current?.getBounds();
      if (bounds?.isValid()) instance.fitBounds(bounds, { padding: [28, 28], maxZoom: 16, animate: false });
    });
    observer.observe(container.current);
    return () => {
      observer.disconnect();
      instance.remove();
      map.current = null;
      route.current = null;
    };
  }, []);

  useEffect(() => {
    const instance = map.current, layers = route.current;
    if (!instance || !layers) return;
    layers.clearLayers();
    const points: L.LatLngTuple[] = [];
    for (const leg of legs) {
      // GeoJSON is [longitude, latitude]; Leaflet expects [latitude, longitude].
      const coordinates: L.LatLngTuple[] = leg.geometry.coordinates.map(([lon, lat]) => [lat, lon]);
      for (const point of coordinates) points.push(point);
      L.polyline(coordinates, {
        color: leg.properties.druh === "WALK" ? "#a67728" : "#17694e",
        weight: 5, opacity: 0.95,
        dashArray: leg.properties.druh === "WALK" ? "6 5" : undefined,
        className: "route-line",
      }).addTo(layers);
    }
    if (points.length === 0) return;
    for (const [index, point] of [points[0], points[points.length - 1]].entries()) {
      L.circleMarker(point, { radius: 6, color: "#fff", weight: 2,
        fillColor: index === 0 ? "#a67728" : "#17694e", fillOpacity: 1,
      }).bindTooltip(index === 0 ? "Start" : "Škola", { permanent: true, direction: "top" }).addTo(layers);
    }
    instance.fitBounds(layers.getBounds(), { padding: [28, 28], maxZoom: 16, animate: false });
  }, [legs]);

  return <figure className="route-geometry">
    <div ref={container} className="student-route-map" role="region" aria-label="Mapa vybraného spojení" />
    {tileError && <p className="data-note" role="status">Mapový podklad se nepodařilo načíst. Trasa a úseky spojení zůstávají dostupné.</p>}
    <div className="route-map-controls"><button type="button" onClick={() => {
      const bounds = route.current?.getBounds();
      if (bounds?.isValid()) map.current?.fitBounds(bounds, { padding: [28, 28], maxZoom: 16 });
    }}>Zobrazit celou trasu</button></div>
    <figcaption>Zelená: doprava · přerušovaná: chůze</figcaption>
  </figure>;
}
