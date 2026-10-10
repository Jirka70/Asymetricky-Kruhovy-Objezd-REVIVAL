#!/usr/bin/env python3
"""Regenerate simulation golden cases using core_job_spec/simulace_oboru.py; run from repo root."""
import sys,json,random,copy
from pathlib import Path
sys.path.insert(0,str(Path('core_job_spec').resolve()))
from simulace_oboru import Data,Nabidka,simuluj
rng=random.Random(20261010)
cases=[]
for index in range(32):
 schools={f'60000000{n}':f'School {n}' for n in range(1,5)}
 offers=[{'redizo':s,'kod_oboru':'23-68-H/01','kapacita':rng.choice([0,2,10,30]),'prihlasky':rng.choice([0,20,80,130])} for s in list(schools)[:2]]
 offers+=[dict(offers[0]),{'redizo':'600000004','kod_oboru':'18-20-M/01','kapacita':20,'prihlasky':120}]
 areas=[{'code':f'{n:06d}','name':f'ZSJ {n}','municipality':'555738','municipality_name':'Test municipality','orp':'4101','orp_name':'Test ORP','children':rng.choice([0,0.2,1.4,5,25,60]),'population':100*n} for n in range(1,9)]
 times=[[a['code'],s,rng.choice([None,0,30,30.125,45,60,60.4,121,150])] for a in areas for s in schools]
 target='600000003' if index%4 else '600000001'
 if index==0:
  for o in offers[:3]: o['kapacita']=30; o['prihlasky']=20
 if index==1: offers[0]['kapacita']=offers[1]['kapacita']=0
 if index==2:
  for o in offers:o['prihlasky']=0
 params={'redizo':target,'obor':'23-68-H/01','kapacita':rng.choice([1,10,30,100]),'max_min':rng.choice([30,60,120]),'scenar':'rano','uroven':'zsj','format':'slovnik'}
 inp={'program_exists':True,'schools':schools,'offers':offers,'areas':areas,'times':times}
 data=Data([Nabidka(o['redizo'],o['kod_oboru'],o['kapacita'],o['prihlasky']) for o in offers],{(a,s):t for a,s,t in times},{a['code']:a['children'] for a in areas},{a['code']:a['name'] for a in areas},schools)
 expected=simuluj(data,params['redizo'],params['obor'],params['kapacita'],params['max_min'])
 cases.append({'input':inp,'params':params,'expected':expected})
Path('backend/tests/fixtures/simulation_reference.json').write_text(json.dumps(cases,ensure_ascii=False,indent=2)+'\n')
print('Oracle verdicts:',sorted({c['expected']['souhrn']['verdikt'] if c['expected']['souhrn'] else 'capacity_sufficient' for c in cases}))
