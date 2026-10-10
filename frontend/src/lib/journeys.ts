import type { Snapshot, School } from "./data";
export type Journey = {
  school: School;
  duration: number;
  departure: string;
  arrival: string;
};
export const minutes = (clock: string) => {
  const [h, m, s] = clock.split(":").map(Number);
  return h * 60 + m + (s || 0) / 60;
};
export const clock = (value: number) =>
  `${String(Math.floor(value / 60)).padStart(2, "0")}:${String(Math.floor(value % 60)).padStart(2, "0")}`;
export function findJourneys(
  data: Snapshot,
  origin: string,
  ids: Set<string>,
  mode: "arrival" | "departure",
  requestedTime: string,
): Journey[] {
  const boundary = minutes(requestedTime);
  return data.schools
    .filter((s) => ids.has(s.id))
    .flatMap((s) => {
      const r = data.routes[origin]?.[s.id];
      if (!r) return [];
      if (
        mode === "arrival" ? minutes(r[2]) > boundary : minutes(r[1]) < boundary
      )
        return [];
      return [{ school: s, duration: r[0], departure: r[1], arrival: r[2] }];
    })
    .sort((a, b) => a.duration - b.duration);
}
export type Leg = {
  kind: "walk" | "ride" | "transfer";
  label: string;
  start: number;
  end: number;
};
/** Illustrative segmentation only: the snapshot does not include OTP legs. */
export function exampleLegs(journey: Journey): Leg[] {
  const start = minutes(journey.departure),
    end = minutes(journey.arrival),
    total = end - start;
  if (total <= 10)
    return [{ kind: "walk", label: "Chůze ke škole", start, end }];
  const walk = Math.min(4, total * 0.15),
    wait = total > 35 ? 5 : 0,
    ride = total - 2 * walk - wait;
  const steps: [Leg["kind"], string, number][] = [
    ["walk", "Chůze na zastávku", walk],
    ["ride", "Jízda veřejnou dopravou", wait ? ride * 0.6 : ride],
    ...(wait
      ? ([
          ["transfer", "Přestup a čekání", wait],
          ["ride", "Navazující spoj", ride * 0.4],
        ] as [Leg["kind"], string, number][])
      : []),
    ["walk", "Chůze ke škole", walk],
  ];
  let cursor = start;
  return steps.map(([kind, label, length], i) => {
    const leg = {
      kind,
      label,
      start: cursor,
      end: i === steps.length - 1 ? end : cursor + length,
    };
    cursor = leg.end;
    return leg;
  });
}
