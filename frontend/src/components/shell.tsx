"use client";
import Link from "next/link";
import Image from "next/image";
import { usePathname } from "next/navigation";
import { MapPinIcon, ArrowUpRightIcon } from "@phosphor-icons/react";
export function Header() {
  const pathname = usePathname();
  return (
    <header className="site-header">
      <a className="skip-link" href="#main">
        Přejít k obsahu
      </a>
      <div className="header-inner">
        <Link href="/kraj" className="brand" aria-label="Obor na dosah – úvod">
          <Image
            src="/logo.svg"
            alt=""
            width={74}
            height={44}
            className="brand-logo"
            unoptimized
          />
          <span>
            Obor na dosah
            <span className="brand-caption">VZDĚLÁVÁNÍ · DOPRAVA · REGION</span>
          </span>
        </Link>
        <nav aria-label="Hlavní navigace">
          <Link
            href="/rodiny"
            aria-current={pathname === "/rodiny" ? "page" : undefined}
          >
            Pro rodiny
          </Link>
          <Link
            href="/kraj"
            aria-current={pathname === "/kraj" ? "page" : undefined}
          >
            Pro kraj
          </Link>
        </nav>
        <div className="header-region">
          <MapPinIcon size={18} />
          <span>Karlovarský kraj</span>
          <span className="preview-label">Pracovní verze</span>
        </div>
      </div>
    </header>
  );
}
export function PageIntro({ family = false }: { family?: boolean }) {
  return (
    <div className="page-intro">
      <div>
        <p className="eyebrow">
          {family ? "VÝBĚR ŠKOLY" : "PLÁNOVÁNÍ VZDĚLÁVACÍ NABÍDKY"}
        </p>
        <h1>
          {family
            ? "Najděte cestu k vysněnému oboru"
            : "Přibližte vzdělání dětem v kraji"}
        </h1>
        <p className="intro-description">
          {family
            ? "Co vás láká studovat? Najděte školy s vaším oborem a porovnejte, jak se do nich dostanete."
            : "Zjistěte, odkud je cesta do školy složitá a komu by pomohla změna nabídky oborů."}
        </p>
      </div>
      <a className="text-link intro-link" href="/metodika">
        O datech a metodice <ArrowUpRightIcon size={17} />
      </a>
    </div>
  );
}
export function Footer() {
  return (
    <footer className="site-footer">
      <span>
        <strong>Obor na dosah</strong>
        <span className="footer-separator">/</span>Vzdělávání blíž lidem.
      </span>
      <span>
        Datový snímek · říjen 2026{" "}
        <a href="/metodika">
          Zdroje a metodika <ArrowUpRightIcon size={14} />
        </a>
      </span>
    </footer>
  );
}
