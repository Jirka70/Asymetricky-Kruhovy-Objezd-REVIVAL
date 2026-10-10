"use client";
import {
  PersonSimpleWalkIcon,
  BusIcon,
  ArrowsLeftRightIcon,
} from "@phosphor-icons/react";
import { clock, exampleLegs, minutes, type Journey } from "@/lib/journeys";

export default function InlineJourney({
  journey,
  boundary,
  mode,
}: {
  journey: Journey;
  boundary: string;
  mode: "arrival" | "departure";
}) {
  return (
    <div className="inline-journey">
      <ol
        aria-label={`Ukázkové úseky cesty do školy ${journey.school.shortName}`}
      >
        {exampleLegs(journey).map((leg, i) => {
          const Icon =
            leg.kind === "walk"
              ? PersonSimpleWalkIcon
              : leg.kind === "ride"
                ? BusIcon
                : ArrowsLeftRightIcon;
          return (
            <li key={i}>
              <Icon size={18} aria-hidden="true" />
              <span>
                <b>
                  {leg.kind === "walk"
                    ? "Chůze"
                    : leg.kind === "ride"
                      ? "Jízda"
                      : "Přestup"}
                </b>
                <small>
                  {Math.round(leg.end - leg.start)} min · {clock(leg.start)}
                </small>
              </span>
            </li>
          );
        })}
      </ol>
      <div className="journey-arrival">
        <span>
          Příchod <strong>{journey.arrival.slice(0, 5)}</strong>
        </span>
        {mode === "arrival" && (
          <small>
            Rezerva {Math.floor(minutes(boundary) - minutes(journey.arrival))}{" "}
            min
          </small>
        )}
      </div>
      <p className="data-note">
        Úseky jsou ukázkové. Čas odchodu a příchodu je z uloženého výpočtu.
      </p>
    </div>
  );
}
