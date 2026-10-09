-- Source: scripts/sql/insert_profesni_skupiny.sql
-- Diesel manages this migration transaction.
-- MVP pracovního trhu. Vygenerováno; NEAPLIKOVÁNO na projektovou DB.
-- Generátor: scripts/generate_pracovni_trh_mvp.py (bez připojení k DB).
-- Pořadí: insert_zamestnavatele.sql -> insert_profesni_skupiny.sql -> insert_poptavka_profesi.sql -> insert_obor_profese.sql.
-- OBORY musí být předem naplněné. Použijte psql -X -v ON_ERROR_STOP=1 -f SOUBOR.
-- SHA-256 datasety/volna-mista_karlovarsky_kraj.json: 78a97818bc6b2f2106b6c7a01f59ed05dc442ace72e2e6e83bcccf6b742dae1c
-- SHA-256 datasety/PZ2026_kolo1_skolobory_prihlasky_karlovarsky_kraj.csv: 461c951c5e52479b6eb44780995aaddeb04f3ff4d8b621c7909885c4fd6a241b
-- SHA-256 datasety/nsp_mapovani_mvp.json: 79349687b74ea52a8a57a7c0ea61be87ef1f422ee42ccfcf0e6f9fe4c640675b
-- SHA-256 datasety/ruian_pracoviste_mvp.json: b886d4f98a62d3c64429608f7a8815b0b3aeb97a14a32ffe3b2ad4e126b4b388

SET LOCAL client_encoding = 'UTF8';
SET LOCAL standard_conforming_strings = on;

CREATE TABLE IF NOT EXISTS public."PROFESNI_SKUPINY" (
    cz_isco3 text PRIMARY KEY CHECK (cz_isco3 ~ '^[0-9]{3}$'),
    nazev text NOT NULL
);
INSERT INTO public."PROFESNI_SKUPINY" (cz_isco3, nazev) VALUES
    ('121', 'Řídící pracovníci v oblasti správy podniku, administrativních a podpůrných činností'),
    ('122', 'Řídící pracovníci v oblasti obchodu, marketingu, výzkumu, vývoje, reklamy a styku s veřejností'),
    ('131', 'Řídící pracovníci v zemědělství, lesnictví, rybářství a v oblasti životního prostředí'),
    ('132', 'Řídící pracovníci v průmyslové výrobě, těžbě, stavebnictví, dopravě a v příbuzných oborech'),
    ('134', 'Řídící pracovníci v oblasti vzdělávání, zdravotnictví, v sociálních a jiných oblastech'),
    ('142', 'Řídící pracovníci v maloobchodě a velkoobchodě'),
    ('213', 'Specialisté v biologických a příbuzných oborech'),
    ('214', 'Specialisté ve výrobě, stavebnictví a příbuzných oborech'),
    ('221', 'Lékaři (kromě zubních lékařů)'),
    ('222', 'Všeobecné sestry a porodní asistentky se specializací'),
    ('226', 'Ostatní specialisté v oblasti zdravotnictví'),
    ('232', 'Učitelé odborných předmětů, praktického vyučování, odborného výcviku (kromě pro žáky se speciálními vzdělávacími potřebami) a lektoři dalšího vzdělávání'),
    ('233', 'Učitelé na středních školách (kromě odborných předmětů), konzervatořích a na 2. stupni základních škol'),
    ('234', 'Učitelé na 1. stupni základních škol a učitelé v oblasti předškolní výchovy'),
    ('235', 'Ostatní specialisté v oblasti výchovy a vzdělávání'),
    ('241', 'Specialisté v oblasti financí'),
    ('242', 'Specialisté v oblasti strategie a personálního řízení'),
    ('243', 'Specialisté v oblasti prodeje, nákupu, marketingu a styku s veřejností'),
    ('252', 'Specialisté v oblasti databází a počítačových sítí'),
    ('263', 'Specialisté v oblasti sociální, církevní a v příbuzných oblastech'),
    ('311', 'Technici ve fyzikálních a průmyslových oborech'),
    ('312', 'Mistři a příbuzní pracovníci v oblasti těžby, výroby a stavebnictví'),
    ('314', 'Technici v biologických oborech a příbuzných oblastech'),
    ('322', 'Všeobecné sestry a porodní asistentky bez specializace'),
    ('324', 'Veterinární technici a asistenti'),
    ('325', 'Ostatní odborní pracovníci v oblasti zdravotnictví'),
    ('331', 'Odborní pracovníci v ekonomických a příbuzných oborech'),
    ('333', 'Zprostředkovatelé služeb'),
    ('334', 'Odborní administrativní pracovníci a asistenti'),
    ('335', 'Pracovníci veřejné správy v oblasti státních regulací'),
    ('341', 'Odborní pracovníci v oblasti právní, sociální a církevní'),
    ('342', 'Odborní pracovníci v oblasti sportu a fitness'),
    ('343', 'Odborní pracovníci v oblasti umění a kultury, šéfkuchaři'),
    ('351', 'Technici provozu a uživatelské podpory informačních a komunikačních technologií a příbuzní pracovníci'),
    ('352', 'Technici v oblasti telekomunikací a vysílání'),
    ('411', 'Všeobecní administrativní pracovníci'),
    ('413', 'Pracovníci pro zadávání dat a zpracování textů'),
    ('421', 'Pokladníci ve finančních institucích, bookmakeři, půjčovatelé peněz, inkasisté pohledávek a pracovníci v příbuzných oborech'),
    ('422', 'Pracovníci informačních služeb'),
    ('431', 'Úředníci pro zpracování číselných údajů'),
    ('432', 'Úředníci v logistice'),
    ('441', 'Ostatní úředníci'),
    ('511', 'Obslužní pracovníci, průvodčí v osobní dopravě a průvodci v cestovním ruchu'),
    ('512', 'Kuchaři (kromě šéfkuchařů), pomocní kuchaři'),
    ('513', 'Číšníci, servírky, barmani a příbuzní pracovníci'),
    ('514', 'Kadeřníci, kosmetici a pracovníci v příbuzných oborech'),
    ('515', 'Provozní pracovníci'),
    ('516', 'Ostatní pracovníci v oblasti osobních služeb'),
    ('521', 'Stánkoví a pouliční prodavači potravin'),
    ('522', 'Provozovatelé maloobchodních a velkoobchodních prodejen, prodavači a příbuzní pracovníci v prodejnách'),
    ('524', 'Ostatní pracovníci v oblasti prodeje'),
    ('531', 'Pracovníci péče o děti, asistenti pedagogů'),
    ('532', 'Pracovníci osobní péče ve zdravotní a sociální oblasti'),
    ('541', 'Pracovníci v oblasti ochrany a ostrahy'),
    ('611', 'Zahradníci a pěstitelé'),
    ('612', 'Chovatelé zvířat pro trh'),
    ('711', 'Řemeslníci a kvalifikovaní pracovníci hlavní stavební výroby'),
    ('712', 'Řemeslníci a kvalifikovaní pracovníci při dokončování staveb'),
    ('713', 'Malíři a příbuzní pracovníci, pracovníci povrchového čištění budov'),
    ('721', 'Slévači, svářeči a příbuzní pracovníci'),
    ('722', 'Kováři, nástrojaři a příbuzní pracovníci'),
    ('723', 'Mechanici a opraváři strojů a zařízení (kromě elektrických)'),
    ('731', 'Pracovníci v oblasti uměleckých a tradičních řemesel'),
    ('732', 'Pracovníci polygrafie'),
    ('741', 'Montéři, mechanici a opraváři elektrických zařízení'),
    ('742', 'Mechanici a opraváři elektronických přístrojů a komunikačních technologií'),
    ('751', 'Výrobci a zpracovatelé potravin a příbuzní pracovníci'),
    ('752', 'Zpracovatelé dřeva, truhláři (kromě stavebních) a příbuzní pracovníci'),
    ('753', 'Výrobci oděvů, výrobků z kůží a kožešin a pracovníci v příbuzných oborech'),
    ('812', 'Obsluha zařízení na zpracování a povrchovou úpravu kovů a jiných materiálů'),
    ('814', 'Obsluha strojů na výrobu a zpracování výrobků z pryže, plastu a papíru'),
    ('815', 'Obsluha strojů na výrobu a úpravu textilních, kožených a kožešinových výrobků'),
    ('816', 'Obsluha strojů na výrobu potravin a příbuzných výrobků'),
    ('817', 'Obsluha strojů a zařízení na zpracování dřeva a výrobu papíru'),
    ('818', 'Ostatní obsluha stacionárních strojů a zařízení'),
    ('821', 'Montážní dělníci výrobků a zařízení'),
    ('832', 'Řidiči motocyklů a automobilů (kromě nákladních)'),
    ('833', 'Řidiči nákladních automobilů, autobusů a tramvají'),
    ('834', 'Obsluha pojízdných zařízení'),
    ('911', 'Uklízeči a pomocníci v domácnostech, hotelích, administrativních, průmyslových a jiných objektech'),
    ('912', 'Pracovníci pro ruční mytí vozidel, oken, praní prádla a příbuzní pracovníci'),
    ('921', 'Pomocní pracovníci v zemědělství, lesnictví a rybářství'),
    ('931', 'Pomocní pracovníci v oblasti těžby a stavebnictví'),
    ('932', 'Pomocní pracovníci ve výrobě'),
    ('933', 'Pomocní pracovníci v dopravě a skladování'),
    ('941', 'Pomocní pracovníci při přípravě jídla'),
    ('961', 'Pracovníci s odpady'),
    ('962', 'Ostatní pomocní pracovníci')
ON CONFLICT (cz_isco3) DO UPDATE SET nazev = EXCLUDED.nazev;

