"use client";
import { useState, useSyncExternalStore, type ReactNode } from "react";
import { CaretDownIcon, MapTrifoldIcon } from "@phosphor-icons/react";

function subscribe(callback: () => void) {
  const query = window.matchMedia("(min-width: 1001px)");
  query.addEventListener("change", callback);
  return () => query.removeEventListener("change", callback);
}
const desktopSnapshot = () => window.matchMedia("(min-width: 1001px)").matches;
const serverSnapshot = () => false;

export default function MapPanel({
  id,
  title,
  subtitle,
  children,
}: {
  id: string;
  title: string;
  subtitle: string;
  children: ReactNode;
}) {
  const desktop = useSyncExternalStore(
    subscribe,
    desktopSnapshot,
    serverSnapshot,
  );
  const [open, setOpen] = useState(false);
  return (
    <aside className="comparison-map" aria-labelledby={id}>
      <div className="compact-map-heading">
        <h2 id={id}>{title}</h2>
        <p>{subtitle}</p>
      </div>
      {!desktop && (
        <button
          className="mobile-map-toggle"
          aria-expanded={open}
          aria-controls={`${id}-content`}
          onClick={() => setOpen(!open)}
        >
          <MapTrifoldIcon size={18} />
          {open ? "Skrýt mapu" : "Zobrazit mapu a školy"}
          <CaretDownIcon size={14} />
        </button>
      )}
      <div id={`${id}-content`} hidden={!desktop && !open}>
        {(desktop || open) && children}
      </div>
    </aside>
  );
}
