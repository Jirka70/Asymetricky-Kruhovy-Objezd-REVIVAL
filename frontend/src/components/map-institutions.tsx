"use client";
import { useEffect, useRef, useState } from "react";
import type { EChartsType } from "echarts/core";
import { GraduationCapIcon, BriefcaseIcon, XIcon } from "@phosphor-icons/react";
import {
  clusterPlaces,
  clusterZoom,
  MAX_MAP_ZOOM,
  type MapPlace,
  type PlaceGroup,
  type PlaceKind,
  type ProjectedPlace,
} from "@/lib/map-clusters";

const titles: Record<PlaceKind, string> = {
  "school-offered": "Školy s oborem",
  "school-other": "Školy bez oboru",
  employer: "Zaměstnavatelé",
};
function PlaceIcon({ kind }: { kind: PlaceKind }) {
  return kind === "employer" ? (
    <BriefcaseIcon size={17} weight="fill" />
  ) : (
    <GraduationCapIcon size={19} weight="fill" />
  );
}
export default function MapInstitutions({
  chart,
  places,
  onSelect,
}: {
  chart: EChartsType;
  places: MapPlace[];
  onSelect: (place: MapPlace) => void;
}) {
  const [groups, setGroups] = useState<PlaceGroup[]>([]);
  const [nearby, setNearby] = useState<MapPlace[] | null>(null);
  const [announcement, setAnnouncement] = useState("");
  const layer = useRef<HTMLDivElement>(null);
  const lastTrigger = useRef<HTMLButtonElement | null>(null);
  const popup = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let frame = 0;
    function update() {
      frame = 0;
      if (chart.isDisposed()) return;
      const points = places.flatMap((place) => {
        const pixel = chart.convertToPixel({ geoIndex: 0 }, [
          place.lon,
          place.lat,
        ]);
        // Only count institutions whose actual location is in the viewport.
        if (
          !pixel ||
          pixel[0] < 0 ||
          pixel[1] < 0 ||
          pixel[0] > chart.getWidth() ||
          pixel[1] > chart.getHeight()
        )
          return [];
        return [{ ...place, x: pixel[0], y: pixel[1] }];
      });
      const next = clusterPlaces(points, {
        width: chart.getWidth(),
        height: chart.getHeight(),
      });
      setGroups((previous) =>
        JSON.stringify(previous) === JSON.stringify(next) ? previous : next,
      );
    }
    function schedule() {
      if (!frame) frame = requestAnimationFrame(update);
    }
    chart.on("rendered", schedule);
    chart.on("georoam", schedule);
    schedule();
    return () => {
      cancelAnimationFrame(frame);
      chart.off("rendered", schedule);
      chart.off("georoam", schedule);
    };
  }, [chart, places]);

  function closeNearby() {
    setNearby(null);
    requestAnimationFrame(() => {
      if (lastTrigger.current?.isConnected)
        lastTrigger.current.focus({ preventScroll: true });
      else layer.current?.focus({ preventScroll: true });
    });
  }
  function activate(members: ProjectedPlace[], trigger: HTMLButtonElement) {
    if (members.length === 1) {
      setNearby(null);
      onSelect(members[0]);
      return;
    }
    lastTrigger.current = trigger;
    const zoom = (chart.getOption().geo as { zoom: number }[])[0].zoom;
    const sameLocation = members.every(
      (p) =>
        Math.abs(p.lon - members[0].lon) < 0.00001 &&
        Math.abs(p.lat - members[0].lat) < 0.00001,
    );
    if (zoom >= MAX_MAP_ZOOM - 0.1 || (sameLocation && zoom >= 4)) {
      setNearby(members);
      requestAnimationFrame(() =>
        popup.current?.focus({ preventScroll: true }),
      );
      return;
    }
    const next = clusterZoom(
      members,
      zoom,
      chart.getWidth(),
      chart.getHeight(),
    );
    chart.setOption({
      geo: {
        zoom: next.zoom,
        center: chart.convertFromPixel({ geoIndex: 0 }, next.pixelCenter),
      },
    });
    setAnnouncement(
      `Přiblížena skupina: ${titles[members[0].kind]}, počet ${members.length}.`,
    );
    requestAnimationFrame(() => layer.current?.focus({ preventScroll: true }));
  }
  const currentNearby =
    nearby
      ?.map((p) => places.find((current) => current.key === p.key))
      .filter((p): p is MapPlace => !!p) ?? [];
  return (
    <>
      <div
        className="institution-layer"
        ref={layer}
        tabIndex={-1}
        role="group"
        aria-label="Instituce na mapě. Čísla udávají počet institucí; skupinu přiblížíte kliknutím."
      >
        {groups.map((group) => (
          <div
            className={`institution-group${group.categories.length > 1 ? " mixed" : ""}`}
            key={group.key}
            style={{ left: group.x, top: group.y, width: group.width }}
          >
            {group.categories.map(({ kind, members, width }) => {
              const single = members.length === 1;
              const selected = members.some((p) => p.selected);
              return (
                <button
                  key={kind}
                  className={`institution-marker ${kind}${single ? " single" : " clustered"}${selected ? " selected" : ""}`}
                  style={{ width }}
                  data-kind={kind}
                  data-count={members.length}
                  aria-label={
                    single
                      ? `${members[0].name} · ${titles[kind]} · zobrazit detail`
                      : `${titles[kind]}: ${members.length} · přiblížit skupinu`
                  }
                  title={
                    single
                      ? `${members[0].name}\n${members[0].hint ?? titles[kind]}`
                      : `${titles[kind]}: ${members.length}\nKliknutím přiblížíte`
                  }
                  onClick={(event) => activate(members, event.currentTarget)}
                >
                  <PlaceIcon kind={kind} />
                  {group.members.length > 1 && (
                    <strong>{members.length}</strong>
                  )}
                </button>
              );
            })}
          </div>
        ))}
      </div>
      <span className="sr-only" role="status" aria-live="polite">
        {announcement}
      </span>
      {currentNearby.length > 0 && (
        <div
          className="cluster-place-picker"
          role="dialog"
          aria-labelledby="cluster-picker-title"
          ref={popup}
          tabIndex={-1}
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.stopPropagation();
              closeNearby();
            }
          }}
        >
          <div className="cluster-picker-heading">
            <div>
              <h3 id="cluster-picker-title">Instituce blízko sebe</h3>
              <p>Vyberte konkrétní školu nebo pracoviště.</p>
            </div>
            <button onClick={closeNearby} aria-label="Zavřít výběr institucí">
              <XIcon size={18} />
            </button>
          </div>
          <ul>
            {currentNearby.map((place) => (
              <li key={place.key}>
                <button
                  onClick={() => {
                    setNearby(null);
                    onSelect(place);
                  }}
                >
                  <span className={`picker-kind ${place.kind}`}>
                    <PlaceIcon kind={place.kind} />
                  </span>
                  <span>
                    <strong>{place.name}</strong>
                    <small>
                      {place.city} · {place.hint ?? titles[place.kind]}
                    </small>
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </>
  );
}
