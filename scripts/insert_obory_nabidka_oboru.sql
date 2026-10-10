-- PostgreSQL: obory a nabídky škol Karlovarského kraje, 1. kolo 2026.
-- Zdroj: datasety/PZ2026_kolo1_skolobory_prihlasky_karlovarsky_kraj.csv
-- Zdroj přijatých: CERMAT PZ2026_kolo1_skolobory_vysledky.xlsx (sloupec PŘIJATÍ), viz níže.
-- SHA-256 zdroje: 461c951c5e52479b6eb44780995aaddeb04f3ff4d8b621c7909885c4fd6a241b
-- 140 datových řádků CSV (bez hlavičky) = 140 nabídek; 81 oborů; 30 REDIZO.
--
-- Mapování:
--   OBORY.kod / nazev = KKOV / OBOR - NÁZEV.
--   NABIDKA_OBORU.id = ID_SOF (UUID); ID_SO není unikátní a jako PK se nepoužívá.
--   redizo = REDIZO, uložené jako text beze změny; nejde o IZO.
--   kod_oboru = KKOV; display_name = ZAMĚŘENÍ OBORU (prázdné -> NULL).
--   forma_studia = FORMA VZDĚLÁVÁNÍ (zdrojové kódy den / dal).
--   delka_studia = DÉLKA STUDIA v letech; povolený rozsah 1 až 16.
--   pocet_prijimanych = KAPACITA pro rok 2026 (součet 3572).
--   loni_pocet_prihlasek = PŘIHLÁŠKY CELKEM za rok 2026 (součet 9489),
--     podle zadání uživatele; název sloupce zde tedy neoznačuje rok 2025.
--   loni_pocet_prijatych = PŘIJATÍ z CERMAT PZ2026_kolo1_skolobory_vysledky.xlsx
--     (https://data.cermat.cz/files/files/JPZ/agregovana_data_skoly/PZ2026_kolo1_skolobory_vysledky.xlsx),
--     párováno podle ID_SOF; všech 140 nabídek nalezeno, součet 3012 přijatých.
--     Výsledkový soubor uvádí u dvou nabídek SŠ stravování a služeb K. Vary (65-51-H/01)
--     o 1 přihlášku méně (103 a 92); loni_pocet_prihlasek zde zůstává podle souboru přihlášek.
--
-- Každý řádek CSV vytváří jednu nabídku v rozsahu požadovaných sloupců.
-- Obory se sloučí podle kódu. Žádná nabídka se neslučuje ani nevynechává.
-- Existující PK se přeskočí; u nich se jen doplní loni_pocet_prijatych, pokud je NULL.
-- Import neporovnává ostatní počty ani hodnoty se zdrojem.
-- Existující tabulky musí mít níže uvedené sloupce a kompatibilní typy/omezení.
-- Samostatný skript nepotřebuje při spuštění původní CSV.
-- Spuštění z kořene projektu:
-- psql -h 127.0.0.1 -U obor -d obor_na_dosah -v ON_ERROR_STOP=1 -f scripts/insert_obory_nabidka_oboru.sql

BEGIN;
SET LOCAL client_encoding = 'UTF8';
SET LOCAL standard_conforming_strings = on;

CREATE TABLE IF NOT EXISTS public."OBORY" (
    kod text PRIMARY KEY,
    nazev text NOT NULL
);

CREATE TABLE IF NOT EXISTS public."NABIDKA_OBORU" (
    id uuid PRIMARY KEY,
    redizo text NOT NULL,
    kod_oboru text NOT NULL REFERENCES public."OBORY" (kod),
    display_name text NULL,
    forma_studia text NOT NULL,
    delka_studia integer NOT NULL CHECK (delka_studia BETWEEN 1 AND 16),
    pocet_prijimanych integer NOT NULL CHECK (pocet_prijimanych >= 0),
    loni_pocet_prihlasek integer NOT NULL CHECK (loni_pocet_prihlasek >= 0),
    loni_pocet_prijatych integer NULL CHECK (loni_pocet_prijatych >= 0)
);

-- Dočasná tabulka uchová všechny zdrojové řádky pro import.
CREATE TEMP TABLE import_nabidka_oboru_2026 (
    id uuid PRIMARY KEY,
    redizo text NOT NULL,
    kod_oboru text NOT NULL,
    nazev_oboru text NOT NULL,
    display_name text NULL,
    forma_studia text NOT NULL,
    delka_studia integer NOT NULL,
    pocet_prijimanych integer NOT NULL,
    loni_pocet_prihlasek integer NOT NULL,
    loni_pocet_prijatych integer NULL
) ON COMMIT DROP;

INSERT INTO pg_temp.import_nabidka_oboru_2026 (
    id, redizo, kod_oboru, nazev_oboru, display_name, forma_studia,
    delka_studia, pocet_prijimanych, loni_pocet_prihlasek, loni_pocet_prijatych
) VALUES
    ('a2270111-15f3-4732-bb35-ec3e870a1c64', '600008975', '79-41-K/81', 'Gymnázium', NULL, 'den', 8, 30, 63, 30),
    ('4f796583-d06a-4c4b-8ab4-5c46ad3d547a', '600008991', '63-41-M/02', 'Obchodní akademie', NULL, 'den', 4, 30, 106, 30),
    ('17a993ab-7681-4ba5-bb44-7ac36753a62f', '600008991', '79-41-K/41', 'Gymnázium', NULL, 'den', 4, 30, 90, 28),
    ('e36360b3-a6f1-47c8-af28-78fb33f702a1', '600008991', '79-41-K/81', 'Gymnázium', NULL, 'den', 8, 30, 81, 30),
    ('548bbc5a-38b4-4c6f-b319-a8ddb59d1e31', '600009009', '79-41-K/41', 'Gymnázium', 'Gymnázium Cheb - Brána na VŠ', 'den', 4, 32, 73, 30),
    ('731b2a5f-bd59-4b0b-9c7c-365ddf551a21', '600009009', '79-41-K/81', 'Gymnázium', 'Gymnázium Cheb - Brána na VŠ', 'den', 8, 60, 140, 61),
    ('e73f811f-11cf-4d57-865a-19247cb9937c', '600009033', '65-42-M/01', 'Hotelnictví', 'Hotelnictví a turismus', 'den', 4, 90, 139, 68),
    ('1ed73658-64be-44da-9b85-599d7f0abca6', '600009033', '65-51-H/01', 'Kuchař - číšník', 'Kuchař/kuchařka', 'den', 3, 30, 49, 8),
    ('17870544-8665-4edf-b5d3-11bbe0a488bf', '600009033', '65-51-H/01', 'Kuchař - číšník', 'Číšník/servírka', 'den', 3, 30, 67, 22),
    ('d78161fb-f0c9-484b-81aa-3478b592b46a', '600009076', '79-41-K/81', 'Gymnázium', NULL, 'den', 8, 18, 76, 18),
    ('6af4055b-a628-404c-a65d-f2e04bf10ad5', '600009084', '18-20-M/01', 'Informační technologie', NULL, 'den', 4, 30, 95, 30),
    ('3850c927-ccb8-4f20-8f0f-451099f301c5', '600009084', '23-41-M/01', 'Strojírenství', NULL, 'den', 4, 30, 66, 29),
    ('5d79cf42-7ce2-4fa0-9c6c-db1474b90114', '600009084', '23-68-H/01', 'Mechanik opravář motorových vozidel', NULL, 'den', 3, 30, 131, 30),
    ('8a35d392-84c6-4282-b900-91997c0a3a98', '600009084', '26-41-M/01', 'Elektrotechnika', NULL, 'dal', 5, 30, 5, 1),
    ('f21ef7e0-0224-4e30-8119-3c913b0ef7fe', '600009084', '26-41-M/01', 'Elektrotechnika', NULL, 'den', 4, 30, 129, 29),
    ('da9009b8-55ab-4849-a7f1-9a05c8cab911', '600009084', '39-41-L/01', 'Autotronik', NULL, 'den', 4, 30, 122, 30),
    ('3fb378f2-5ca5-47c4-87c6-34106704562b', '600009084', '68-43-M/01', 'Veřejnosprávní činnost', NULL, 'den', 4, 30, 109, 30),
    ('2978c9eb-eacb-47ed-95d3-81ccf496dbea', '600009084', '78-42-M/01', 'Technické lyceum', NULL, 'den', 4, 30, 67, 29),
    ('4a673635-50c0-4fe4-8eff-61a65aa3b807', '600009106', '29-54-H/01', 'Cukrář', NULL, 'den', 3, 15, 53, 15),
    ('95b96fbf-7ded-4b46-b99c-e40d8062ec87', '600009106', '65-41-L/01', 'Gastronomie', '(Gastronomie a hotelnictví)', 'den', 4, 30, 91, 30),
    ('1ac03a87-9e3c-4ca5-85b1-77bbb6fdead1', '600009106', '65-41-L/51', 'Gastronomie', '(Gastronomie a hotelnictví nástavba)', 'den', 2, 30, 52, 30),
    ('269d523f-d42b-4106-82d6-a7d40db2f87c', '600009106', '65-51-E/01', 'Stravovací a ubytovací služby', NULL, 'den', 3, 24, 51, 17),
    ('ce0eb7d8-bd90-459a-8365-a91640133816', '600009106', '65-51-H/01', 'Kuchař - číšník', '(Kuchař)', 'den', 3, 60, 104, 58),
    ('71d787be-27a6-4f0d-9cf1-449d3ad5da01', '600009106', '65-51-H/01', 'Kuchař - číšník', '(Číšník, barman, barista)', 'den', 3, 30, 109, 29),
    ('f6642b70-af29-4a7e-97ce-41c9b6d951c1', '600009106', '65-51-H/01', 'Kuchař - číšník', '(Číšník)', 'den', 3, 30, 93, 29),
    ('6e1ebb5e-0333-4eee-9936-949ba60caee1', '600009106', '69-51-H/01', 'Kadeřník', NULL, 'den', 3, 15, 101, 15),
    ('62f6a440-fd2b-4eb9-90f0-dc1fe48f4800', '600009122', '75-31-M/01', 'Předškolní a mimoškolní pedagogika', NULL, 'den', 4, 60, 147, 60),
    ('062372a0-66e1-46b5-a61f-f5b5104e64d3', '600009122', '75-41-M/01', 'Sociální činnost', NULL, 'den', 4, 30, 121, 30),
    ('bf772c56-0cdc-492a-b788-6381564450f5', '600009122', '78-42-M/03', 'Pedagogické lyceum', NULL, 'den', 4, 30, 150, 30),
    ('ab256e9b-1f08-4071-a590-d086d965364a', '600009122', '79-41-K/41', 'Gymnázium', NULL, 'den', 4, 30, 76, 30),
    ('ee504940-2639-44aa-995c-ab1a0d8bb5f1', '600009190', '63-41-M/02', 'Obchodní akademie', NULL, 'den', 4, 30, 142, 30),
    ('da328179-2f04-4db6-b48d-5f15dc8eb4b5', '600009190', '65-42-M/02', 'Cestovní ruch', NULL, 'den', 4, 60, 263, 60),
    ('2024d4cf-ff2a-4818-9f34-6a139df534c1', '600009203', '16-01-M/01', 'Ekologie a životní prostředí', NULL, 'den', 4, 15, 51, 14),
    ('e14fe422-aec9-4969-ad59-b9383dbaac19', '600009203', '28-44-M/01', 'Aplikovaná chemie', NULL, 'den', 4, 15, 33, 14),
    ('2e89a3a0-2d6c-4cd3-ad91-63363bd1eec0', '600009203', '28-57-H/01', 'Výrobce a dekoratér keramiky', NULL, 'den', 3, 5, 21, 5),
    ('951d6877-e946-4bb2-8ab8-5f43d56b499b', '600009203', '28-58-H/01', 'Sklář - výrobce a zušlechťovatel skla', NULL, 'den', 3, 5, 23, 5),
    ('0faf2b65-00aa-482c-85f5-9b4761b2b7d1', '600009203', '82-41-M/02', 'Užitá fotografie a média', NULL, 'den', 4, 15, 36, 13),
    ('bfcc4561-1d43-40d9-ae35-1c43ce02d0b5', '600009203', '82-41-M/05', 'Grafický design', NULL, 'den', 4, 15, 56, 15),
    ('04c1d223-7657-4da9-b626-787674df3d42', '600009203', '82-41-M/07', 'Modelářství a návrhářství oděvů', NULL, 'den', 4, 12, 29, 12),
    ('8de98c95-2dc7-43c8-a6c8-12b8b0a652da', '600009203', '82-41-M/12', 'Výtvarné zpracování keramiky a porcelánu', NULL, 'den', 4, 10, 22, 7),
    ('169f63b4-7234-41f9-8bd9-ceeadfff1050', '600009203', '82-41-M/13', 'Výtvarné zpracování skla a světelných objektů', NULL, 'den', 4, 8, 7, 2),
    ('02b05377-accf-4c6c-a7f2-5aa196bbec24', '600009211', '63-41-M/02', 'Obchodní akademie', 'Cestovní ruch', 'den', 4, 30, 100, 29),
    ('806da292-5f9e-43d9-984f-58cdafd276e3', '600009211', '63-41-M/02', 'Obchodní akademie', 'Právo a finance', 'den', 4, 22, 87, 22),
    ('98688b53-e34e-41bd-b983-e9a7b46a2bc6', '600009238', '23-68-H/01', 'Mechanik opravář motorových vozidel', NULL, 'den', 3, 19, 55, 6),
    ('e21a2fbe-993d-4f81-8055-d05c5f076af2', '600009238', '26-51-H/02', 'Elektrikář - silnoproud', NULL, 'den', 3, 10, 31, 9),
    ('9441da18-76fb-4096-b173-1bc31bf9a01a', '600009238', '33-56-H/01', 'Truhlář', NULL, 'den', 3, 10, 18, 5),
    ('0b9ebb28-e688-447e-b138-cfab09c2e6f1', '600009238', '36-52-H/01', 'Instalatér', NULL, 'den', 3, 5, 17, 4),
    ('3ee16241-cf79-47ea-af70-d0d664ec0898', '600009238', '39-41-H/01', 'Malíř a lakýrník', NULL, 'den', 3, 5, 10, 0),
    ('f1a47d87-e088-4491-9c12-a4678f75e5ac', '600009238', '39-41-L/01', 'Autotronik', NULL, 'den', 4, 22, 41, 16),
    ('eb75ff22-3bbe-4990-9a01-71b937d6783c', '600009262', '79-41-K/41', 'Gymnázium', NULL, 'den', 4, 30, 81, 30),
    ('c3d7a666-e55d-43f9-9139-c3b39d554c05', '600009262', '79-41-K/81', 'Gymnázium', NULL, 'den', 8, 60, 173, 60),
    ('622a9c8d-df59-4682-9233-fbca84c534e2', '600009271', '41-46-M/01', 'Lesnictví', NULL, 'dal', 4, 10, 21, 10),
    ('9a30dc82-3d6f-4c83-8012-5d1e42664702', '600009271', '41-46-M/01', 'Lesnictví', NULL, 'den', 4, 32, 64, 32),
    ('dbcaa509-2d7c-491b-adbd-2f545538208a', '600009271', '41-52-E/01', 'Zahradnické práce', NULL, 'den', 3, 8, 7, 1),
    ('bcfbc33f-05dc-4087-a836-663ee32e85b0', '600009271', '41-52-H/01', 'Zahradník', NULL, 'den', 3, 12, 21, 10),
    ('fc862983-70b4-45ce-95c0-8ab26a5bcc48', '600009271', '41-56-H/01', 'Lesní mechanizátor', NULL, 'den', 3, 16, 44, 15),
    ('b0e08809-b466-47e2-baf9-2a930d9126b5', '600009297', '79-41-K/41', 'Gymnázium', NULL, 'den', 4, 30, 83, 30),
    ('3b07cb74-5ec6-478a-9341-a038d37f398d', '600009297', '79-41-K/81', 'Gymnázium', NULL, 'den', 8, 60, 156, 58),
    ('7adb80e5-a274-49c7-a950-6fbd252ec32a', '600009301', '37-41-M/01', 'Provoz a ekonomika dopravy', NULL, 'den', 4, 30, 91, 30),
    ('bbb1d7a4-ffa4-4aad-bf1b-47a8039679e8', '600009301', '37-42-M/01', 'Logistické a finanční služby', NULL, 'den', 4, 30, 117, 30),
    ('3b377c82-f9d7-4b92-aa27-7421324ad021', '600009301', '66-53-H/01', 'Operátor skladování', NULL, 'den', 3, 24, 82, 24),
    ('9444f927-9b6f-484d-9370-d2f96792047b', '600009319', '23-51-H/01', 'Strojní mechanik', NULL, 'den', 3, 12, 32, 7),
    ('d87a97fb-2196-4488-b8b0-a25cde2bf0af', '600009319', '26-51-H/02', 'Elektrikář - silnoproud', NULL, 'den', 3, 30, 88, 26),
    ('32fb9c5f-3d87-4ad4-8877-9da53f102e88', '600009319', '33-56-H/01', 'Truhlář', NULL, 'den', 3, 24, 59, 19),
    ('bb21aeb3-53b0-45ae-b929-d4c5230f62c9', '600009319', '36-47-M/01', 'Stavebnictví', NULL, 'den', 4, 30, 80, 29),
    ('b33b0101-ee13-46e1-b0cd-abf581761d83', '600009319', '36-52-H/01', 'Instalatér', NULL, 'den', 3, 24, 81, 24),
    ('fa9745be-8f1b-4db1-a764-e86eb0c74e91', '600009319', '36-67-E/02', 'Stavební práce', NULL, 'den', 2, 24, 7, 4),
    ('c702b4c2-891e-429f-a5d1-dd3602365f75', '600009319', '36-67-H/01', 'Zedník', NULL, 'den', 3, 30, 61, 21),
    ('e362f8ba-1e0e-4bbb-b4dd-1af18d04d1ef', '600009319', '39-41-H/01', 'Malíř a lakýrník', NULL, 'den', 3, 12, 38, 11),
    ('66b9ebd0-fe83-4657-87c2-a34da72a51b8', '600009882', '79-41-K/41', 'Gymnázium', NULL, 'den', 4, 60, 96, 52),
    ('933de324-4ae2-440f-9622-a16986c386ac', '600009882', '79-41-K/61', 'Gymnázium', NULL, 'den', 6, 30, 55, 30),
    ('98432a2d-208a-44fd-8691-f7cfb37f1bd4', '600009882', '79-41-K/81', 'Gymnázium', NULL, 'den', 8, 30, 100, 30),
    ('5f80c8f6-fae8-470b-bef2-192ed0995415', '600009904', '63-41-M/02', 'Obchodní akademie', NULL, 'dal', 4, 12, 18, 7),
    ('50480a4d-8186-416e-b095-dd58b9a3d77e', '600009904', '63-41-M/02', 'Obchodní akademie', 'Informační technologie, Sportovní management, Moderní asistentka', 'den', 4, 50, 119, 47),
    ('3bb26c53-8d71-4d6e-8f17-11d3ab63763c', '600009904', '64-41-L/51', 'Podnikání', NULL, 'dal', 3, 13, 38, 11),
    ('a6c89bcb-5c1b-49a5-bfc1-88e1d94af502', '600019632', '53-41-M/03', 'Praktická sestra', NULL, 'den', 4, 60, 109, 60),
    ('49c5e04f-f703-4332-a62c-eb37fb3b0352', '600019632', '53-41-M/04', 'Masér ve zdravotnictví', NULL, 'den', 4, 15, 52, 15),
    ('9aa068b5-859f-4b94-94ef-b19a8acbaa2e', '600019632', '75-41-M/01', 'Sociální činnost', NULL, 'den', 4, 15, 43, 15),
    ('be85a6ce-df1f-418d-992b-728c1e9933b3', '600019641', '53-41-M/02', 'Nutriční asistent', NULL, 'den', 4, 24, 131, 24),
    ('9e90e92c-5191-49c0-9968-8029590850f8', '600019641', '53-41-M/03', 'Praktická sestra', NULL, 'den', 4, 48, 166, 48),
    ('7234dadf-b1e9-40c4-8327-1833988c9aae', '600019641', '53-41-M/04', 'Masér ve zdravotnictví', NULL, 'den', 4, 24, 74, 24),
    ('d782e262-8889-402c-8372-1bc64d8cfbc6', '600019641', '78-42-M/04', 'Zdravotnické lyceum', NULL, 'den', 4, 24, 122, 24),
    ('7189c5ee-9a7d-4a71-9bea-b71a08724711', '600022854', '69-54-E/01', 'Provozní služby', 'úklid v zařízeních, úklid administrativních ploch nebo veřejných prostor, hospodyně v domácnosti apod., dále výkon pomocných prací při přípravě pokrmů, při praní a žehlení prádla, při šití a opravách prádla, jednoduchých oděvů a při šití bytových doplňků.', 'den', 2, 17, 26, 15),
    ('c290d11c-0062-49d6-a2b7-5ae5a229a893', '600022854', '78-62-C/02', 'Praktická škola dvouletá', 'jednoduché pracovní činnosti v oblasti služeb a výroby (např. v sociálních a komunálních službách, ve zdravotnictví, ve výrobních podnicích, v zemědělství), případně k pokračování v dalším vzdělávání', 'den', 2, 12, 9, 8),
    ('9b19dbba-37ed-4e54-9b1d-175f8d9be526', '600066584', '78-62-C/02', 'Praktická škola dvouletá', 'Praktická škola dvouletá', 'den', 2, 7, 2, 1),
    ('55c9fc07-3bc5-45d7-826d-36d74a49280e', '600073017', '23-51-H/01', 'Strojní mechanik', NULL, 'den', 3, 12, 18, 10),
    ('ddb958eb-2691-448e-b422-9b8ba2e3657d', '600073017', '23-52-H/01', 'Nástrojař', NULL, 'den', 3, 12, 14, 0),
    ('a3f44621-935b-4915-9979-f39f57554775', '600170462', '18-20-M/01', 'Informační technologie', 'Umělá inteligence', 'den', 4, 30, 64, 22),
    ('a0ebf8c9-533b-4b3d-97c5-dc280cfa113b', '600170462', '23-41-M/01', 'Strojírenství', 'Programování CNC strojů', 'den', 4, 23, 35, 7),
    ('47ee08e2-6e50-40d2-9af7-e96c9936e63a', '600170462', '23-51-E/01', 'Strojírenské práce', NULL, 'den', 3, 12, 30, 11),
    ('9915fdea-6982-4432-8c78-f1c240fe1898', '600170462', '23-68-H/01', 'Mechanik opravář motorových vozidel', NULL, 'den', 3, 30, 124, 30),
    ('22d8c7c3-ae8e-4304-9477-61564edaff04', '600170462', '26-41-L/01', 'Mechanik elektrotechnik', 'Elektromechanik pro zařízení a přístroje', 'den', 4, 12, 40, 11),
    ('c51becac-c4d7-48da-947b-52ac074d26b8', '600170462', '26-52-H/01', 'Elektromechanik pro zařízení a přístroje', NULL, 'den', 3, 30, 55, 22),
    ('679c6c6d-ab2f-425c-9678-4ee95e3c963d', '600170462', '29-51-E/01', 'Potravinářská výroba', 'Cukrář', 'den', 3, 16, 42, 16),
    ('1c110189-191d-4648-9cac-1b4ea125cfc4', '600170462', '29-54-H/01', 'Cukrář', NULL, 'den', 3, 12, 59, 12),
    ('5dc1b26d-7cb0-4b72-abb8-cfe9ad4094eb', '600170462', '29-56-H/01', 'Řezník - uzenář', NULL, 'den', 3, 12, 5, 0),
    ('ba087d38-f22a-4f2f-b6fb-f7a531c71467', '600170462', '33-56-H/01', 'Truhlář', NULL, 'den', 3, 23, 56, 19),
    ('dda313af-ea21-4873-bd0f-f856bd6caffe', '600170462', '33-57-E/01', 'Dřevařská výroba', NULL, 'den', 3, 12, 25, 7),
    ('abc98c63-4050-4a26-8c91-b3fce438afab', '600170462', '36-52-H/01', 'Instalatér', NULL, 'den', 3, 23, 73, 22),
    ('2f4dca85-23d7-4b98-80dd-4158b209b872', '600170462', '39-08-M/01', 'Požární ochrana', NULL, 'den', 4, 30, 43, 17),
    ('e152d532-a1f2-460b-b150-28f815697ded', '600170462', '41-41-M/01', 'Agropodnikání', NULL, 'den', 4, 30, 39, 17),
    ('6d0ad7fb-faa7-4e44-9523-6e64723a7785', '600170462', '41-51-E/01', 'Zemědělské práce', NULL, 'den', 3, 12, 29, 11),
    ('9b5cc6f8-a859-4163-a569-021914e284ca', '600170462', '41-51-H/01', 'Zemědělec - farmář', NULL, 'den', 3, 12, 31, 12),
    ('8b7f0e8f-f8b8-4b2d-a066-6e1e9d8e4daf', '600170462', '41-52-H/01', 'Zahradník', NULL, 'den', 3, 12, 23, 9),
    ('99717acc-f9eb-4d55-81ba-b536dcb62911', '600170462', '41-55-H/01', 'Opravář zemědělských strojů', NULL, 'den', 3, 30, 101, 28),
    ('17ad2486-b65a-491e-acb1-6c7bce251a25', '600170462', '63-41-M/01', 'Ekonomika a podnikání', NULL, 'den', 4, 23, 66, 16),
    ('21a068c7-5352-478b-aae0-c33d0f316133', '600170462', '66-51-H/01', 'Prodavač', 'Výrobce lahůdek', 'den', 3, 23, 22, 5),
    ('47bc16af-ae73-4492-a91c-c7f011484754', '600170462', '69-51-H/01', 'Kadeřník', NULL, 'den', 3, 30, 98, 27),
    ('e3244540-1e37-40e7-be25-c83b4b03aca8', '600170462', '82-51-L/06', 'Uměleckořemeslná stavba hudebních nástrojů', NULL, 'den', 4, 12, 10, 6),
    ('a25ba4a1-014f-48ef-93ca-db5e521f26fb', '600170527', '18-20-M/01', 'Informační technologie', NULL, 'den', 4, 30, 89, 30),
    ('b1977b80-8169-4598-98a8-f4219e402d99', '600170527', '23-41-M/01', 'Strojírenství', NULL, 'den', 4, 30, 30, 9),
    ('91457ed4-db70-4a55-bca3-393fcbd29bea', '600170527', '23-45-M/01', 'Dopravní prostředky', NULL, 'den', 4, 30, 24, 5),
    ('10397f77-74a4-464a-8fdb-64cef6002c14', '600170527', '23-51-H/01', 'Strojní mechanik', NULL, 'den', 3, 30, 62, 30),
    ('1fe2138b-fd90-497f-9f15-4d82f3199944', '600170527', '23-56-H/01', 'Obráběč kovů', NULL, 'den', 3, 30, 43, 9),
    ('3b73fcfc-c7bd-49cf-96c3-2b43b082b9a7', '600170527', '23-68-H/01', 'Mechanik opravář motorových vozidel', NULL, 'den', 3, 30, 145, 29),
    ('6d29f327-4653-48b0-b4c3-8d761dee411b', '600170527', '26-41-M/01', 'Elektrotechnika', NULL, 'den', 4, 30, 84, 28),
    ('5f50217c-5c55-4f67-897d-3ca347c5832c', '600170527', '26-51-H/01', 'Elektrikář', NULL, 'den', 3, 30, 118, 29),
    ('86bbe7bb-dd35-495a-ab01-6656d4dba988', '600170527', '63-41-M/01', 'Ekonomika a podnikání', NULL, 'den', 4, 45, 120, 20),
    ('010ef907-f773-4d1f-ba42-0f5365993858', '600170527', '68-43-M/01', 'Veřejnosprávní činnost', NULL, 'den', 4, 45, 117, 27),
    ('b5932f91-6b14-442c-ba42-cdd6a7949f17', '610100718', '79-41-K/41', 'Gymnázium', NULL, 'den', 4, 20, 21, 11),
    ('7e33d4c8-8ec8-4119-8c67-b6ccc71c80ef', '650003969', '68-42-L/51', 'Bezpečnostní služby', NULL, 'dal', 3, 30, 28, 6),
    ('25d056a4-e8bf-40c9-a2ed-0b578bb52f86', '650003969', '68-42-M/01', 'Bezpečnostně právní činnost', NULL, 'den', 4, 60, 189, 60),
    ('913254ac-4231-4d59-951e-ff50c789c14e', '651011434', '41-43-M/02', 'Chovatelství', NULL, 'den', 4, 15, 11, 5),
    ('52951209-cdda-4df4-b9c5-e8549bdb108e', '651011434', '41-53-H/02', 'Jezdec a chovatel koní', NULL, 'den', 3, 15, 17, 6),
    ('46aa035b-0be0-464c-99b9-16f19457c90e', '651012988', '29-51-E/01', 'Potravinářská výroba', '/Cukrářské práce', 'den', 3, 14, 37, 14),
    ('f1337069-2fc8-4e7b-a723-655a8bcbd396', '651012988', '29-51-E/02', 'Potravinářské práce', '/Pekařské práce', 'den', 2, 12, 23, 6),
    ('32dc97ba-f89e-45b2-8578-d1d2c3a9ded2', '651012988', '29-53-H/01', 'Pekař', NULL, 'den', 3, 8, 23, 5),
    ('e7216554-e875-47af-9fbf-28a8c4448eec', '651012988', '29-54-H/01', 'Cukrář', NULL, 'den', 3, 24, 98, 23),
    ('e416b0d2-a122-4eaf-9398-73a8e02da89d', '651012988', '33-56-H/01', 'Truhlář', NULL, 'den', 3, 15, 56, 15),
    ('2a264b9e-b0f1-462a-b819-48e074985238', '651012988', '33-57-E/01', 'Dřevařská výroba', '/Zpracování dřeva', 'den', 3, 12, 27, 12),
    ('cce0a113-ee68-496a-8d70-90810929e4fb', '651012988', '64-41-L/51', 'Podnikání', NULL, 'den', 2, 60, 113, 60),
    ('e7fddc88-4c6e-49a7-8a70-e413c7d78086', '651012988', '65-51-E/02', 'Práce ve stravování', NULL, 'den', 2, 14, 38, 12),
    ('820b6ed3-8fb8-4950-97e9-05ccf69e3967', '651012988', '65-51-H/01', 'Kuchař - číšník', '/Číšník-servírka', 'den', 3, 13, 60, 12),
    ('96c55759-1cb2-4eb4-8463-7c9ace72b2f1', '651012988', '65-51-H/01', 'Kuchař - číšník', '/Kuchař-číšník', 'den', 3, 13, 37, 13),
    ('90c4932a-121c-4561-89b2-c8ca088a3c12', '651012988', '65-51-H/01', 'Kuchař - číšník', '/Kuchař', 'den', 3, 26, 50, 25),
    ('af72b49f-9fe4-4ae3-ac75-456190f582c5', '651012988', '66-51-H/01', 'Prodavač', NULL, 'den', 3, 15, 40, 15),
    ('9169ba3c-b8f8-45d6-a878-28256fe1212b', '651012988', '66-52-H/01', 'Aranžér', NULL, 'den', 3, 23, 54, 17),
    ('56598278-6b91-4923-b9ad-52190205573c', '651012988', '69-41-L/01', 'Kosmetické služby', NULL, 'den', 4, 30, 111, 30),
    ('43f9b236-e8f5-49b6-8ae2-ffa000448200', '651012988', '69-51-H/01', 'Kadeřník', NULL, 'den', 3, 34, 124, 34),
    ('71e1605f-fa8c-442c-91fb-a18d700452be', '651012988', '72-41-M/01', 'Informační služby', NULL, 'den', 4, 30, 54, 22);

INSERT INTO public."OBORY" (kod, nazev)
SELECT DISTINCT kod_oboru, nazev_oboru
FROM pg_temp.import_nabidka_oboru_2026
ON CONFLICT (kod) DO NOTHING;

INSERT INTO public."NABIDKA_OBORU" (
    id, redizo, kod_oboru, display_name, forma_studia,
    delka_studia, pocet_prijimanych, loni_pocet_prihlasek, loni_pocet_prijatych
)
SELECT id, redizo, kod_oboru, display_name, forma_studia,
       delka_studia, pocet_prijimanych, loni_pocet_prihlasek, loni_pocet_prijatych
FROM pg_temp.import_nabidka_oboru_2026
ON CONFLICT (id) DO NOTHING;

-- Doplnění počtu přijatých do nabídek naimportovaných dřívější verzí skriptu (tehdy NULL).
UPDATE public."NABIDKA_OBORU" AS n
SET loni_pocet_prijatych = i.loni_pocet_prijatych
FROM pg_temp.import_nabidka_oboru_2026 AS i
WHERE n.id = i.id
  AND n.loni_pocet_prijatych IS NULL;

COMMIT;
