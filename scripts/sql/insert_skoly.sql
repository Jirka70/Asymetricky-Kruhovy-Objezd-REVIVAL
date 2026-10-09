-- Import škol lze spouštět opakovaně; existující REDIZO se přeskočí.
-- Kódy ZSJ zachovávají počáteční nuly; dříve zkrácené kódy se opraví.
-- Celý import probíhá v jedné transakci, chyba vrátí všechny jeho změny zpět.
-- Pro psql použijte -v ON_ERROR_STOP=1.

BEGIN;
SET LOCAL client_encoding = 'UTF8';
SET LOCAL standard_conforming_strings = on;

CREATE TABLE IF NOT EXISTS stredni_skoly (
    redizo VARCHAR(20) PRIMARY KEY,
    nazev VARCHAR(255),
    kod_zsj VARCHAR(20),
    adresa VARCHAR(255),
    lat DECIMAL(10, 6),
    lon DECIMAL(10, 6),
    web VARCHAR(255)
);

INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600022854', 'Základní škola a střední škola Karlovy Vary, příspěvková organizace', '153851', 'Vančurova 83/2, 36017 Karlovy Vary', 50.248111, 12.831028, 'https://www.specskoly.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600008975', 'Gymnázium Aš, příspěvková organizace', '000540', 'Hlavní 2514/106, 35201 Aš', 50.216618, 12.194203, 'https://www.gymas.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009301', 'Střední škola logistická Dalovice, příspěvková organizace', '024589', 'Hlavní 114/29, 36263 Dalovice', 50.246902, 12.892417, 'https://www.logistickaskola.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600019632', 'Střední zdravotnická škola a vyšší odborná škola Cheb, příspěvková organizace', '050920', 'Hradební 58/10, 35002 Cheb', 50.078765, 12.366493, 'https://www.szsavoscheb.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009009', 'Gymnázium Cheb, příspěvková organizace', '050938', 'Nerudova 2283/7, 35002 Cheb', 50.076275, 12.362412, 'https://www.gymcheb.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600170462', 'Integrovaná střední škola Cheb, příspěvková organizace', '050920', 'Obrněné brigády 2258/6, 35002 Cheb', 50.07735, 12.366284, 'https://www.iss-cheb.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009190', 'Obchodní akademie, vyšší odborná škola cestovního ruchu a jazyková škola s právem státní jazykové zkoušky Karlovy Vary, příspěvková organizace', '063592', 'Bezručova 1312/17, 36001 Karlovy Vary', 50.231556, 12.874274, 'https://www.oakv.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009122', 'Střední pedagogická škola, gymnázium a vyšší odborná škola Karlovy Vary, příspěvková organizace', '063622', 'Lidická 455/40, 36001 Karlovy Vary', 50.232575, 12.88918, 'https://www.pedgym-kv.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009262', 'První české gymnázium v Karlových Varech, příspěvková organizace', '063622', 'Národní 445/25, 36001 Karlovy Vary', 50.233994, 12.887607, 'https://www.gymkvary.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009203', 'Střední uměleckoprůmyslová škola keramická a sklářská Karlovy Vary, příspěvková organizace', '063550', 'nám. 17. listopadu 710/12, 36005 Karlovy Vary', 50.234883, 12.85184, 'https://supskv.cz/')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009106', 'Střední škola stravování a služeb Karlovy Vary, příspěvková organizace', '063452', 'Ondřejská 1122/56, 36001 Karlovy Vary', 50.227672, 12.881997, 'https://www.ssstravovani.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600019641', 'Střední zdravotnická škola a vyšší odborná škola zdravotnická Karlovy Vary, příspěvková organizace', '063444', 'Poděbradská 1247/2, 36001 Karlovy Vary', 50.226864, 12.87576, 'https://www.zdravkakv.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009319', 'Střední odborná škola stavební Karlovy Vary, příspěvková organizace', '063622', 'nám. K. Sabiny 159/16, 36001 Karlovy Vary', 50.236186, 12.885361, 'https://www.stavebniskolakv.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009033', 'Hotelová škola Mariánské Lázně, příspěvková organizace', '091588', 'Komenského 449/2, 35301 Mariánské Lázně', 49.962248, 12.700109, 'https://www.hotelovaskola.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600008991', 'Gymnázium a obchodní akademie Mariánské Lázně, příspěvková organizace', '091570', 'Ruská 355/7, 35301 Mariánské Lázně', 49.973218, 12.701646, 'https://www.goaml.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009084', 'Střední průmyslová škola Ostrov, příspěvková organizace', '308790', 'Klínovecká 1197, 36301 Ostrov', 50.304766, 12.952136, 'https://www.spsostrov.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009297', 'Gymnázium Ostrov, příspěvková organizace', '115916', 'Studentská 1205, 36301 Ostrov', 50.302827, 12.952738, 'https://www.gymostrov.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009882', 'Gymnázium Sokolov a Krajské vzdělávací centrum, příspěvková organizace', '152277', 'Husitská 2053, 35601 Sokolov', 50.176253, 12.640226, 'https://www.gymso.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600170527', 'Integrovaná střední škola technická a ekonomická Sokolov, příspěvková organizace', '152234', 'Jednoty 1620, 35601 Sokolov', 50.182788, 12.644488, 'https://www.isste.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('651012988', 'Střední škola živnostenská Sokolov, příspěvková organizace', '152200', 'Žákovská 716, 35601 Sokolov', 50.186655, 12.641212, 'https://www.zivnostenska-sokolov.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009271', 'Střední lesnická škola Žlutice, příspěvková organizace', '197769', 'Žižkov 345, 36452 Žlutice', 50.096452, 13.16005, 'https://www.slszlutice.cz')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600066584', 'Základní škola a střední škola Aš, příspěvková organizace', '000515', 'Studentská 1612/13, 35201 Aš', 50.228133, 12.183502, 'https://www.zspsas.eu/')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600073017', 'Střední škola, základní škola a mateřská škola Kraslice, příspěvková organizace', '073288', 'Havlíčkova 1717, 35801 Kraslice', 50.33237, 12.515958, 'https://www.skolakraslice.cz/')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009076', 'Svobodná chebská škola, základní škola a gymnázium s.r.o.', '050920', 'Jánské náměstí 256/15, 35001 Cheb', 50.081327, 12.367883, 'http://www.schs.cz/')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009211', 'Soukromá obchodní akademie Podnikatel, spol. s r.o. ', '063703', 'nám. V. Řezáče 136/5, 36009 Karlovy Vary', 50.235134, 12.880333, 'https://soapodnikatel.cz/')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('650003969', 'TRIVIS - Střední škola veřejnoprávní Karlovy Vary, s.r.o. ', '063444', 'T. G. Masaryka 559/1, 36001 Karlovy Vary', 50.228944, 12.874078, 'https://www.trivis-kv.cz/')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009238', 'Střední odborná škola Karlovy Vary, s.r.o.', '308773', 'Konečná 908/21, 36005 Karlovy Vary', 50.240108, 12.852584, 'https://www.soskv.cz/')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('651011434', 'JEZDECKÁ AKADEMIE - střední odborná škola Mariánské Lázně s.r.o.', '091561', 'Mariánské Lázně 569, 35301 Mariánské Lázně', 49.972377, 12.722912, 'http://www.jezdeckaakademie.cz/')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('600009904', 'Soukromá obchodní akademie Sokolov, s.r.o.', '152269', 'Hornická 1569, 35605 Sokolov', 50.17594, 12.658735, 'https://www.e-soas.cz/')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('610100718', 'Škola Můj Projekt Mánesova - Gymnázium, základní škola a mateřská škola s.r.o.', '302449', 'Mánesova 1672, 35601 Sokolov', 50.174063, 12.675284, 'https://www.skolamanesova.cz/')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('691009091', 'Střední škola Euroinstitut v Karlovarském kraji', '000540', 'Plzeňská 1985/10, 35201 Aš', 50.21701, 12.193472, 'https://www.euroinstitut.cz/euroinstitut-karlovarsky-kraj/')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('690000014', 'Vojenská střední škola a Vyšší odborná škola Ministerstva obrany v Moravské Třebové (pracoviště Sokolov)', '152234', 'Jednoty 1620, 35601 Sokolov', 50.182787, 12.645863, 'https://www.vsmt.cz/')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('680000011', 'Vyšší policejní škola a Střední policejní škola Ministerstva vnitra v Praze (pracoviště Sokolov)', '152226', 'Komenského 759, 35601 Sokolov', 50.178288, 12.639017, 'https://www.skolamv.cz/studujici/stredni-policejni-skola-sokolov.html')
ON CONFLICT (redizo) DO NOTHING;
INSERT INTO stredni_skoly (redizo, nazev, kod_zsj, adresa, lat, lon, web) VALUES ('691018821', 'Lyceum Avantech - střední odborná škola, příspěvková organizace', '052175', 'Komenského 273, 35735 Chodov', 50.239075, 12.749481, 'https://avantechchodov.cz/')
ON CONFLICT (redizo) DO NOTHING;

-- Starší import převáděl kódy ZSJ na čísla a ztratil počáteční nuly.
-- Opraví i existující školy, které INSERT při konfliktu REDIZO přeskočil.
UPDATE stredni_skoly
SET kod_zsj = lpad(kod_zsj, 6, '0')
WHERE kod_zsj ~ '^[0-9]{1,5}$';

COMMIT;
