"""Saved-evidence presentation adapters. This module never executes an episode."""
import argparse
import hashlib
import json
import re
import os
from collections import Counter
from pathlib import Path
from urllib.parse import unquote, urlparse

import pd_coast_native as native

REPO=Path(os.environ.get('PD_STUDY_WORKSPACE',str(Path(__file__).resolve().parents[2]))).resolve()
TIMING=REPO/'outputs/timing-reference/loop-20261024T185303Z'
TIMING_REPORT=REPO/'outputs/timing-reference-report-20261024T185303Z'
RL_REPORT=REPO/'outputs/coast-ignite/loop-20261022T173645Z/campaign-c/learning/report-final-v2'
MODES=['first_feasible_v1','minimum_fuel_grid_v1','minimum_fuel_extended_grid_v1']
LABELS=['Legacy first feasible','Fuel-ranked 63 pairs','132-pair reference']
read=lambda p:json.loads(Path(p).read_text())
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()


def write_new(path,value):
    with Path(path).open('x') as f:json.dump(value,f,indent=2,allow_nan=False);f.write('\n')


def metrics(timing):
    result=[{'id':'fuel_used_kg','label':'Mission fuel used','unit':'kg','lower_is_better':True,'successful_pairs_only':True,'measurement':'Initial scenario fuel minus recorded native final fuel; both captured flights must verify target landing'},
        {'id':'contact_speed_mps','label':'Incoming contact speed','unit':'m/s','lower_is_better':True,'successful_pairs_only':True,'measurement':'Recorded native incoming contact before final-state velocity reset; both flights must verify landing'}]
    if timing:
        result += [{'id':key,'label':label,'unit':'s','lower_is_better':None,'successful_pairs_only':True,'measurement':'Recorded selected timing; positive/negative indicates later/longer, without an optimality claim'} for key,label in [('coast_s','Selected coast / preparation'),('duration_s','Selected Poly duration')]]
    return result


def timing():
    original=read(TIMING/'report-spec.json');a=read(TIMING/'analysis.json');rows=[];bindings=[]
    assert all(a[k] for k in ['primary_99_percent_gate_passed','boundary_engineering_gate_passed','regression_gate_passed'])
    regression_ids={r['case']:i for i,r in enumerate(read(TIMING/'regressions/result.json')['rows'])}
    for source in original['rows']:
        folder=Path(source['capture']);e=read(folder/'evaluation.json');p=read(folder/'proof.json');s=e['scenario']
        if folder.is_relative_to(TIMING):
            phase=folder.relative_to(TIMING).parts[0]
            cohort={'development-completion':'development'}.get(phase,phase)
        else:cohort='development'
        mode=p.get('mode')
        if mode not in MODES:
            assert mode=='native_pair_v2', 'Unknown retained timing profile'
            case=folder.name.removesuffix('-native_pair_v2')
            expected=next((TIMING/phase/'summaries'/(case+'-'+MODES[0]+'.json') for phase in ['development','development-completion'] if (TIMING/phase/'summaries'/(case+'-'+MODES[0]+'.json')).exists()),None)
            assert expected is not None and read(expected)['info']==read(folder/'decisions.json'), 'Retained default is not proven equivalent'
        variant=mode if mode in MODES else MODES[0]
        m=re.search(r'-(\d{3})(?:-|$)',s['id'])
        ordinal=int(m.group(1)) if m else regression_ids.get(p.get('case',folder.name.removesuffix('-'+variant)),0)
        family=s.get('metadata',{}).get('family',source['population'].split(' · ')[-1])
        rows.append({'capture':str(folder),'population':cohort.replace('_',' ').title()+' · '+family.replace('_',' '),
            'cohort':cohort,'variant':variant,'mission_index':ordinal,'label':LABELS[MODES.index(variant)],
            'context_html':'<section><h2>Retained timing comparison</h2><p>The selected timing, captured native state and original proof are preserved. Use the study context above to switch the baseline/candidate for this original. Incoming contact is distinct from post-contact zero velocity.</p></section>'})
        bindings.append({'row':len(rows)-1,'capture':str(folder),'capture_sha256':sha(folder/'evaluation.json'),'decisions_sha256':sha(folder/'decisions.json')})
    counts={c:len({read(Path(r['capture'])/'evaluation.json')['physical_sha256'] for r in rows if r['cohort']==c}) for c in sorted({r['cohort'] for r in rows})}
    definitions={'primary':('Original independent confirmation','Frozen 300-draw uniform mixture; now retained exposed evidence'),
        'boundary':('Boundary endpoint panel','Engineering endpoints; no random reliability denominator'),
        'regressions':('Retained arrival regressions','21 handoff witnesses; only the selected reference was scheduled'),
        'development':('Exposed reference selection','Development comparison; separate from independent qualification'),
        'pilot':('Repeated cost pilot','Four exposed originals repeated for cost; not fresh reliability evidence')}
    effects=[]
    contrast_defs=[('expanded',MODES[1],MODES[2],'63 pairs → 132 pairs'),('legacy63',MODES[0],MODES[1],'Legacy → 63 pairs'),('legacy132',MODES[0],MODES[2],'Legacy → 132 pairs')]
    for key,left,right,_ in contrast_defs:
        p=a['populations']['primary']['paired'][left+'__'+right]
        effects.append({'cohort':'primary','contrast':key,'metric':'fuel_used_kg','pairs':p['successful_pairs'],'estimate':-p['mean_saving_kg'],
            'low':-p['mean_saving_kg_ci95'][1],'high':-p['mean_saving_kg_ci95'][0],'method':'Original-level paired percentile bootstrap, 10,000 resamples; frozen source analysis'})
    benchmark=read(TIMING/'benchmark/result.json');performance=[]
    for mode,label in zip(MODES,LABELS):
        values=[r['measurements']['selection_wall_ns'][0]/1e6 for r in benchmark['rows'] if r['mode']==mode and r['role']=='measured']
        performance.append({'variant':mode,'label':label,'unit':'ms','originals':12,'measured_repeats':7,'values':values,
            'scope':'Sequential one-host benchmark on exposed inputs, after first invocation and two warmups. Controller-frame, plant and whole-episode costs are distinct; this is not a 60 Hz deadline guarantee.'})
    study={'schema_version':1,'id':'timing-reference-qualification-v1','question':'Does the larger timing bank improve the landing reference, preserve qualification and justify its planning cost?',
        'motivation':'Earlier apparent intermediate-goal benefits largely disappeared after consistent timing search. This study qualified a fair offline reference before another planner/RL decision. The primary comparison is 63 to 132 pairs; legacy timing is background.',
        'changed':['63 timing pairs → one 132-pair superset; coast and duration change jointly'],
        'fixed':['Same vehicle, initial fuel, Earth gravity, flat plant and Poly burn','Same clocks, native contact predicates, inputs and complete proof contract'],
        'variants':[{'id':m,'label':l} for m,l in zip(MODES,LABELS)],
        'contrasts':[{'id':k,'baseline':left,'candidate':right,'label':label} for k,left,right,label in contrast_defs],
        'cohorts':[{'id':c,'label':definitions[c][0],'role':definitions[c][1],'originals':n} for c,n in counts.items()],
        'metrics':metrics(True),'claims':[
            {'label':'Frozen primary qualification','status':'passed','detail':'132 pairs: 300/300 verified; one-sided 95% lower bound 99.0064% ≥ declared 99% criterion'},
            {'label':'Full primary efficiency · 63 → 132','status':'descriptive','detail':'21.976 kg less mean fuel (3.728%); all 300 pairs verify landing'},
            {'label':'Scope and engineering checks','status':'limited','detail':'60/60 boundary cases and 21/21 retained handoffs pass. Fixed envelope; no default promotion or general controller claim'}],
        'effects':effects,'performance':performance,'links':[{'label':'Frozen analysis','href':'http://starship:8000/timing-reference/loop-20261024T185303Z/analysis.json'},
            {'label':'Original complete edition','href':'http://starship:8000/timing-reference-report-20261024T185303Z/report.html'},
            {'label':'This edition’s source bindings','href':'source-bindings.json'}],
        'default_cohort':'primary','default_contrast':'expanded','default_metric':'fuel_used_kg','condition_axes':['family','sink_band'],'timing_explanation':True}
    return rows,study,bindings


def rl():
    old=read(RL_REPORT/'report-input.json');rows=[];bindings=[]
    assert old['labels']==['RL','Heuristic','Immediate','Never'] and len(old['missions'])==600
    matrix=Counter()
    for mission in old['missions']:
        attempts={a['approach']:a for a in mission['attempts']}
        matrix[(attempts[1]['outcome']=='landed',attempts[0]['outcome']=='landed')]+=1
    assert matrix==Counter({(True,True):443,(False,True):146,(True,False):3,(False,False):8})
    variants=['rl','heuristic','immediate','never']
    for mission in old['missions']:
        for attempt in sorted(mission['attempts'],key=lambda a:a['approach']):
            evidence=read(RL_REPORT/(Path(attempt['href']).stem+'-evidence.json'))
            evaluation=(RL_REPORT/unquote(urlparse(evidence['source_evaluation_href']).path)).resolve()
            assert evaluation.is_relative_to(REPO/'outputs')
            assert sha(evaluation)==evidence['source_evaluation_sha256']
            e=read(evaluation);height=e['scenario']['initial_state']['position_m']['y'];band=250*int(height//250)
            variant=variants[attempt['approach']]
            rows.append({'capture':str(evaluation.parent),'population':f'Supplied airborne arcs · initial height {band}–{band+250} m',
                'cohort':'confirmation','variant':variant,'mission_index':mission['mission_index'],'occurrence':mission['occurrence'],'label':old['labels'][attempt['approach']],
                'context_html':'<section><h2>Retained coast and ignition comparison</h2><p>The original mission supplies its target-directed arc, prepared attitude and absolute touchdown time. The saved policy decides when to activate the unchanged Poly burn. This presentation executes no policy or native flight. Incoming contact and the final clamped state have separate meanings.</p></section>'})
            bindings.append({'row':len(rows)-1,'capture':str(evaluation.parent),'capture_sha256':sha(evaluation),'decisions_sha256':sha(evaluation.parent/'decisions.json')})
    study={'schema_version':1,'id':'coast-ignite-paired-confirmation-v1','question':'Did learned ignition improve landing outcomes, and which baseline landings did it lose?',
        'motivation':'The original bounded hybrid compared a frozen ignition policy with a validation-selected fixed delay. Headline success rates hide individual gains and losses; this retained study keeps the exact paired originals visible.',
        'changed':['Ignition decision: fixed heuristic delay versus frozen learned policy','Immediate and never-ignite controls remain available as secondary contrasts'],
        'fixed':['Same 600 supplied arcs, vehicle, prepared attitude, touchdown time and Poly burn','Initial-height 250 m bands and horizontal-speed 25 m/s bands are descriptive display bins only'],
        'variants':[{'id':v,'label':label} for v,label in zip(variants,old['labels'])],
        'contrasts':[{'id':'heuristic-rl','label':'Heuristic → RL','baseline':'heuristic','candidate':'rl'},
            {'id':'immediate-rl','label':'Immediate → RL','baseline':'immediate','candidate':'rl'},
            {'id':'never-rl','label':'Never ignite → RL','baseline':'never','candidate':'rl'}],
        'cohorts':[{'id':'confirmation','label':'Original fresh confirmation · retained','role':'600 once-independent originals from the frozen study; no new flight or qualification','originals':600}],
        'metrics':metrics(False),'claims':[
            {'label':'Frozen landing comparison','status':'descriptive','detail':'RL 589/600 versus heuristic 446/600 verified landings'},
            {'label':'Paired outcomes','status':'descriptive','detail':'146 gained · 3 lost · 443 preserved · 8 neither landed; net +143'},
            {'label':'Remaining limitations','status':'limited','detail':'Lost baseline landings 355, 436 and 514 remain explicit. Supplied-arc task; no general RL/controller qualification'}],
        'effects':[],'performance':[],'links':[{'label':'Original complete edition','href':'http://starship:8000/coast-ignite/loop-20261022T173645Z/campaign-c/learning/report-final-v2/report.html'},
            {'label':'This edition’s source bindings','href':'source-bindings.json'}],
        'default_cohort':'confirmation','default_contrast':'heuristic-rl','default_metric':'fuel_used_kg','condition_axes':['initial_height_band_m','initial_horizontal_speed_band_mps'],'timing_explanation':False}
    return rows,study,bindings


def main():
    parser=argparse.ArgumentParser();parser.add_argument('root',type=Path);parser.add_argument('study',choices=['timing','rl']);parser.add_argument('output',type=Path);parser.add_argument('--pilot',action='store_true');args=parser.parse_args()
    rows,study,bindings=(timing if args.study=='timing' else rl)()
    if args.pilot:
        wanted={0,1,2} if args.study=='timing' else {0,6,64,355,436,514}
        selected=[i for i,r in enumerate(rows) if r['mission_index'] in wanted and r['cohort']==study['default_cohort']]
        rows=[rows[i] for i in selected];bindings=[bindings[i] for i in selected]
        for i,b in enumerate(bindings):b['row']=i
        study['cohorts']=[{**c,'originals':len(wanted)} for c in study['cohorts'] if c['id']==study['default_cohort']]
        study['effects']=[];study['claims']=[{'label':'Software pilot subset','status':'descriptive','detail':f'{len(wanted)} retained originals for presentation acceptance; not a new population or qualification'}]
    output=args.output.resolve();root=args.root.resolve();suffix=args.study+('-pilot' if args.pilot else '-full')
    spec={'title':('Timing reference' if args.study=='timing' else 'Learned ignition')+' · study and mission review',
        'subtitle':'Question → declared comparison → original missions → rich captured evidence',
        'summary':'Integrated study context, paired originals and unchanged complete source evidence.',
        'rows':rows,'grouped':True,'focus_flat':True,'max_bytes':9*1024**3,'study':study}
    write_new(root/(suffix+'-spec.json'),spec)
    native.render_report(json.dumps(spec,allow_nan=False),str(output))
    source_declarations=[TIMING/'analysis.json',TIMING/'reference-freeze.json',TIMING/'benchmark/result.json'] if args.study=='timing' else [RL_REPORT/'report-input.json']
    write_new(output/'source-bindings.json',{'schema_version':1,'study':study['id'],'renderer_sha256':sha(__file__),
        'native_library_sha256':sha(native.__file__),'spec_sha256':sha(root/(suffix+'-spec.json')),'native_transitions':0,'bindings':bindings})
    write_new(output/'source-declarations.json',[{'path':str(p),'sha256':sha(p)} for p in source_declarations])
    write_new(root/(suffix+'-render.json'),{'output':str(output),'captures':len(rows),'cohorts':study['cohorts'],
        'native_transitions':0,'bytes':sum(p.stat().st_size for p in output.iterdir() if p.is_file())})
    print(json.dumps({'study':suffix,'captures':len(rows),'output':str(output)}))


if __name__=='__main__':main()
