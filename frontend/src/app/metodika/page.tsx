import Link from "next/link";
export const metadata = { title: "Data a metodika · Obor na dosah" };
export default function Methodology() {
  return (
    <main id="main" className="page methodology">
      <p className="eyebrow">JAK ČÍST VÝSLEDKY</p>
      <h1>Co vám výsledky řeknou</h1>
      <p className="intro-description">
        Podívejte se, z čeho vycházíme a co vzít v úvahu při porovnávání škol.
      </p>
      <div className="methodology-content">
        <section>
          <h2>Katalogy škol a uložené dojezdy</h2>
          <p>
            Školy, nabídku oborů, kapacity, přihlášky a pracovní poptávku načítáme
            z datové služby projektu. Dojezdy, geometrie a demografie pocházejí
            z uloženého snímku. Simulace používá aktuální nabídku škol a tyto
            uložené dojezdy. Aplikace nevyhledává nové spoje v OTP.
          </p>
        </section>
        <section>
          <h2>Dojezdy ze ZSJ do škol</h2>
          <p>
            Matice obsahuje 28 526 dvojic: 839 základních sídelních jednotek ×
            34 škol. Výpočet platí pro pondělí 12. října 2026, příjezd 7:00–8:00
            a odjezd ve stejný den. Doba zahrnuje chůzi a čekání. Jde o
            nejkratší z vrácených kandidátů OTP, nikoli zaručené optimum všech
            spojení.
          </p>
          <p>
            Výchozí bod je modelový bod uvnitř ZSJ, nikoli konkrétní adresa
            dítěte. „Bez spojení“ znamená, že výpočet v tomto okně nenašel
            použitelnou variantu. Neznamená to, že se do školy nedá dostat
            nikdy. Při filtrování konkrétního času pracujeme jen s již uloženou
            variantou pro každou dvojici; další varianty nedohledáváme.
          </p>
          <p>
            Zdroj: <code>datasety/zsj_skoly_2026-10-12.csv</code> a soubor{" "}
            <code>_diagnostics.csv</code>. Časy zobrazené v minutách
            zaokrouhlujeme nahoru; grafy a výpočty pracují s přesnějšími
            hodnotami.
          </p>
        </section>
        <section>
          <h2>Mapa a podíly dětí</h2>
          <p>
            Mapa zobrazuje základní sídelní jednotky (ZSJ) z importované
            geografické vrstvy. Geometrie jsou pro zobrazení zjednodušené.
          </p>
          <p>
            Podíly jsou vážené odhadovaným počtem dětí 10–14 let, odvozeným ze
            SLDB 2021 a věkové struktury obcí. Nejde o aktuální počet žáků ani
            přesné individuální bydliště. V kraji jde o modelový součet 15 837
            dětí. Území bez odhadované populace do populačních podílů
            nepřispívají.
          </p>
          <p>
            ZSJ barvíme podle doby cesty k nejbližší škole s vybraným oborem. Ve
            změnové mapě ukazujeme zkrácení či prodloužení času. Nastavená
            časová hranice určuje podíl dětí s dostupnou školou ve statistikách.
          </p>
          <p>
            Blízké instituce seskupujeme podle přiblížení mapy. Zelená čísla
            označují školy s oborem, červená školy bez oboru a modrá pracoviště
            zaměstnavatelů. Číslo počítá instituce, nikoli pracovní nabídky.
            Kliknutím skupinu přiblížíte; u bodů na stejné poloze lze vybrat
            konkrétní instituci ze seznamu.
          </p>
        </section>
        <section>
          <h2>Simulace nabídky oborů</h2>
          <p>
            Přidání nebo odebrání oboru mění množinu škol, z nichž pro každou
            ZSJ vybíráme nejrychlejší uložený dojezd. Nemění jízdní řády,
            kapacity škol, počty uchazečů ani chování rodin. Jde o model
            dostupnosti ve stávajících školách. Při změně oboru nebo formy
            studia se scénář resetuje.
          </p>
        </section>
        <section>
          <h2>Pracovní místa</h2>
          <p>
            Zobrazujeme pracovní místa v profesích navázaných na vybraný obor.
            Vynecháváme nabídky vyžadující vyšší odborné nebo vysokoškolské
            vzdělání. Propojení oborů s profesními skupinami CZ-ISCO je orientační. Stejná
            nabídka může být relevantní pro více oborů, proto sloupce nesčítáme.
            Poptávka není zárukou uplatnění absolventa. V mapě zobrazujeme pouze
            pracoviště se souřadnicemi. Grafy zahrnují i místa bez známé polohy.
          </p>
          <p>
            Zdroje: tabulky ZAMESTNAVATELE, POPTAVKA_PROFESI a OBOR_PROFESE v
            místní databázi.
          </p>
        </section>
        <section>
          <h2>Podíly přijatých: pouze ukázka</h2>
          <p>
            Skutečné počty přijatých jsou v tabulce NABIDKA_OBORU prázdné. Grafy
            i sloupce označené „Modelová ukázka“ nebo „ukázka“ proto používají
            ilustrační hodnoty. Nejde o skutečnou statistiku školy ani
            pravděpodobnost přijetí konkrétního uchazeče.
          </p>
          <p>
            Kapacita a počty přihlášek jsou skutečné údaje z importu 2026.
            Jejich poměr nezaměňujeme za acceptance rate. Sloupec databáze
            loni_pocet_prihlasek podle zdrojového importu obsahuje přihlášky v
            1. kole 2026.
          </p>
        </section>
        <section>
          <h2>Časová osa: skutečné konce, modelové úseky</h2>
          <p>
            Celkové odjezdy a příjezdy jsou ze souboru diagnostiky. Snímek
            neobsahuje jednotlivé jízdy, přestupy, linky ani zastávky. Barevné
            úseky a detail cesty proto slouží pouze jako návrh budoucího
            rozhraní a nejsou použitelným cestovním itinerářem. V mapě se trasy
            nevykreslují.
          </p>
        </section>
        <div className="methodology-links">
          <Link href="/kraj">← Zpět na mapu kraje</Link>
          <Link href="/rodiny">Prozkoumat školy pro rodiny →</Link>
        </div>
      </div>
    </main>
  );
}
