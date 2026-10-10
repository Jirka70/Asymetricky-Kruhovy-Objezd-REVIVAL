"use client";
import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import {
  CaretDownIcon,
  CheckIcon,
  MagnifyingGlassIcon,
} from "@phosphor-icons/react";
type Option = { value: string; label: string };
const normalize = (v: string) =>
  v
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLocaleLowerCase("cs");
export default function SearchSelect({
  label,
  icon,
  options,
  value,
  onChange,
  className = "",
  hint,
  placeholder = "Začněte psát název…",
}: {
  label: string;
  icon?: ReactNode;
  options: Option[];
  value: string;
  onChange: (v: string) => void;
  className?: string;
  hint?: string;
  placeholder?: string;
}) {
  const id = useId();
  const input = useRef<HTMLInputElement>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const tokens = normalize(query).split(/\s+/).filter(Boolean);
  const matched = options.filter((o) =>
    tokens.every((t) => normalize(o.label).includes(t)),
  );
  const shown = matched.slice(0, 30);
  useEffect(() => {
    if (open)
      document
        .getElementById(`${id}-${active}`)
        ?.scrollIntoView({ block: "nearest" });
  }, [active, open, id]);
  const selected = options.find((o) => o.value === value);
  function choose(o: Option) {
    onChange(o.value);
    setOpen(false);
    setQuery("");
    input.current?.focus();
  }
  return (
    <div
      className={`field search-select ${className}`}
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget)) {
          setOpen(false);
          setQuery("");
        }
      }}
    >
      <label htmlFor={id}>
        {icon}
        {label}
      </label>
      <div className="combobox-input">
        <input
          id={id}
          ref={input}
          role="combobox"
          aria-expanded={open}
          aria-controls={open ? `${id}-list` : undefined}
          aria-describedby={hint ? `${id}-hint` : undefined}
          aria-autocomplete="list"
          aria-activedescendant={
            open && shown[active] ? `${id}-${active}` : undefined
          }
          autoComplete="off"
          value={open ? query : (selected?.label ?? "")}
          placeholder={placeholder}
          title={selected?.label}
          onFocus={(e) => e.currentTarget.select()}
          onClick={() => {
            setOpen(true);
            setActive(0);
          }}
          onChange={(e) => {
            setQuery(e.target.value);
            setOpen(true);
            setActive(0);
          }}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setOpen(true);
              setActive(
                open ? Math.max(0, Math.min(active + 1, shown.length - 1)) : 0,
              );
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setActive(Math.max(0, active - 1));
            } else if (e.key === "Enter" && open) {
              e.preventDefault();
              if (shown[active]) choose(shown[active]);
            } else if (e.key === "Escape") {
              e.preventDefault();
              setOpen(false);
              setQuery("");
            }
          }}
        />
        <span aria-hidden="true">
          {open ? (
            <MagnifyingGlassIcon size={15} />
          ) : (
            <CaretDownIcon size={13} />
          )}
        </span>
      </div>
      {hint && (
        <span className="field-hint" id={`${id}-hint`}>
          {hint}
        </span>
      )}
      <span className="sr-only" role="status">
        {open
          ? `${matched.length} možností. Vyberte šipkami a potvrďte Enterem.`
          : ""}
      </span>
      {open && (
        <div className="combobox-panel">
          <ul id={`${id}-list`} role="listbox" aria-label={label}>
            {shown.map((o, i) => (
              <li
                id={`${id}-${i}`}
                key={o.value}
                role="option"
                aria-selected={o.value === value}
                className={i === active ? "active" : ""}
                onMouseDown={(e) => e.preventDefault()}
                onMouseEnter={() => setActive(i)}
                onClick={() => choose(o)}
              >
                {o.label}
                {value === o.value && <CheckIcon size={13} />}
              </li>
            ))}
          </ul>
          {!shown.length && (
            <p>Nenalezeno. Zkuste kratší název nebo jiný výraz.</p>
          )}
          {matched.length > 30 && (
            <p>{matched.length} možností. Upřesněte výběr zadáním názvu.</p>
          )}
        </div>
      )}
    </div>
  );
}
