"""Export the existing local snapshot. Read-only DB access; no downloads or OTP calls."""
import csv
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
from db_common import connection

db = connection('Export the existing hackathon snapshot for the frontend')

def query(sql):
    return json.loads(db.execute('BEGIN READ ONLY; SELECT coalesce(json_agg(q),\'[]\'::json) FROM (' + sql + ') q; COMMIT;'))

def write(name, value):
    path = ROOT / 'frontend/public/data' / name
    path.write_text(json.dumps(value, ensure_ascii=False, separators=(',', ':')))
    print(f'{name}: {path.stat().st_size:,} bytes')

zsj = query('''SELECT z.kod AS id, z.nazev AS name, z.kod_obce AS municipality,
 round(z.lon::numeric,5) AS lon, round(z.lat::numeric,5) AS lat,
 coalesce(d.populace,0) AS children
 FROM "ZSJ" z LEFT JOIN "DATA_DEMOGRAFIE_ZSJ" d ON d.kod_zsj=z.kod
 AND d.rok=2021 AND d.demo_skupina='1300100014' ORDER BY z.kod''')
# Names retained from the same local RÚIAN source as insert_zsj.sql.
names = json.loads((ROOT / 'frontend/scripts/municipality-names.json').read_text())
municipalities = query('''SELECT kod_obce AS id FROM "ZSJ" GROUP BY kod_obce ORDER BY kod_obce''')
for m in municipalities:
    local=[z for z in zsj if z['municipality']==m['id']]
    m['name']=names.get(m['id'], max(local,key=lambda z:z['children'])['name'].split('-')[0])
    m['children']=sum(z['children'] for z in local)
    m['nameDerived']=m['id'] not in names
schools=query('SELECT redizo AS id,nazev AS name,adresa AS address,lat,lon,web FROM stredni_skoly ORDER BY redizo')
short={'600170462':'ISŠ Cheb','600170527':'ISŠTE Sokolov','600009084':'SPŠ Ostrov','651012988':'SŠ živnostenská Sokolov','600009271':'SLŠ Žlutice','600009301':'SŠ logistická Dalovice','600009319':'SOŠ stavební Karlovy Vary','600009106':'SŠ stravování a služeb KV','600073017':'SŠ Kraslice'}
for s in schools:
    s['city']=s['address'].split(', ')[-1][6:]
    s['shortName']=short.get(s['id'],s['name'].split(',')[0].replace('Základní škola','ZŠ'))
fields=query('SELECT kod AS id,nazev AS name FROM "OBORY" ORDER BY nazev')
offerings=query('''SELECT redizo AS school,kod_oboru AS field,forma_studia AS form,pocet_prijimanych AS capacity,
 loni_pocet_prihlasek AS applications,loni_pocet_prijatych AS accepted FROM "NABIDKA_OBORU"''')
demand=query('''SELECT b.kod_oboru AS field,sum(p.pocet_mist)::integer AS jobs,count(DISTINCT z.ico)::integer AS employers
 FROM "POPTAVKA_PROFESI" p JOIN "OBOR_PROFESE" b USING(cz_isco3) JOIN "ZAMESTNAVATELE" z ON z.id=p.zamestnavatel_id
 GROUP BY b.kod_oboru ORDER BY jobs DESC''')
employers=query('''SELECT z.id::text AS id,z.nazev AS name,z.ico,z.kod_obce AS municipality,
 z.kod_adresniho_mista AS addressPoint,z.lat,z.lon,b.kod_oboru AS field,
 sum(p.pocet_mist)::integer AS jobs,max(p.importovano_at) AS importedAt,
 json_agg(json_build_object('code',p.cz_isco3,'name',g.nazev,'education',p.min_vzdelani,
 'jobs',p.pocet_mist,'suitability',b.vhodnost) ORDER BY b.vhodnost,p.cz_isco3,p.min_vzdelani) AS professions
 FROM "ZAMESTNAVATELE" z JOIN "POPTAVKA_PROFESI" p ON z.id=p.zamestnavatel_id
 JOIN "OBOR_PROFESE" b USING(cz_isco3) JOIN "PROFESNI_SKUPINY" g USING(cz_isco3)
 WHERE z.lat IS NOT NULL AND z.lon IS NOT NULL
 GROUP BY z.id,z.nazev,z.ico,z.kod_obce,z.kod_adresniho_mista,z.lat,z.lon,b.kod_oboru
 ORDER BY sum(p.pocet_mist) DESC,z.id,b.kod_oboru''')
for employer in employers:
    employer['city']=names.get(employer['municipality'], employer['municipality'])
routes={}
with (ROOT/'datasety/zsj_skoly_2026-10-12_diagnostics.csv').open(encoding='utf-8-sig') as f:
    for r in csv.DictReader(f):
        if r['status']=='ok':
            routes.setdefault(r['kod_zsj'],{})[r['redizo']]=[float(r['doba_jizdy']),r['odjezd'][11:19],r['prijezd'][11:19]]
geo=query('''SELECT kod AS id,ST_AsGeoJSON(ST_SimplifyPreserveTopology(boundary,0.00012),5)::json AS geometry FROM "ZSJ" ORDER BY kod''')
muni_geo=query('''SELECT kod_obce AS id,ST_AsGeoJSON(ST_SimplifyPreserveTopology(ST_UnaryUnion(ST_Collect(boundary)),0.00015),5)::json AS geometry FROM "ZSJ" GROUP BY kod_obce''')
for filename,features in [('zsj.geojson',geo),('municipalities.geojson',muni_geo)]:
    write(filename,{'type':'FeatureCollection','features':[{'type':'Feature','properties':{'name':f['id']},'geometry':f['geometry']} for f in features]})
write('snapshot.json',{'zsj':zsj,'municipalities':municipalities,'schools':schools,'fields':fields,'offerings':offerings,'demand':demand,'employers':employers,'routes':routes,'meta':{'date':'2026-10-12','arrivalWindow':'07:00–08:00','demographyYear':2021,'totalJobs':1889,'totalEmployers':390,'unlocatedWorkplaces':28}})
