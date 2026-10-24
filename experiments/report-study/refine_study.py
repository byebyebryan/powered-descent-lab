"""Render the declared refinement from retained captures and checked sources."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import pd_coast_native as native
from render_studies import timing,rl,TIMING,RL_REPORT,REPO,write_new,sha
from compare_runs import join

PRIOR={'timing':REPO/'outputs/timing-study-review-v2-20261024T212129Z','rl':REPO/'outputs/ignition-study-review-v2-20261024T212129Z'}


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('root',type=Path);p.add_argument('kind',choices=['timing','rl']);p.add_argument('output',type=Path);p.add_argument('--pilot',action='store_true');a=p.parse_args()
    root=a.root.resolve();out=a.output.resolve();rows,study,bindings=(timing if a.kind=='timing' else rl)();old=PRIOR[a.kind];prior=json.loads((old/'study.json').read_text())
    if a.pilot:
        wanted={0,11,30,50} if a.kind=='timing' else {0,6,64,355,436,514}
        selected=[i for i,r in enumerate(rows) if r['mission_index'] in wanted and r['cohort']==study['default_cohort']]
        if a.kind=='timing':selected.append(next(i for i,r in enumerate(rows) if r['cohort']=='regressions'))
        rows=[rows[i] for i in selected];bindings=[{**bindings[i],'row':j} for j,i in enumerate(selected)]
        counts={c['id']:len({(r['mission_index'],r.get('occurrence',0)) for r in rows if r['cohort']==c['id']}) for c in study['cohorts']}
        study['cohorts']=[{**c,'originals':counts[c['id']]} for c in study['cohorts'] if counts[c['id']]]
        study['effects']=[];study['claims']=[{'label':'Saved software prototype','status':'descriptive','detail':'Labelled retained subset for presentation checks, not a new qualification.'}]
    source=sha(old/'report-input.json');variant_ids={v['id'] for v in study['variants']};contract=hashlib.sha256(b''.join((REPO/'pd-core/src'/n).read_bytes() for n in ['sim.rs','model.rs'])).hexdigest()
    # Check each variant's join against its explicitly pinned source. The future
    # native render is bound to the exact same per-row capture/proof records.
    selected_observations=[]
    for r in rows:
        capture=sha(Path(r['capture'])/'evaluation.json');matches=[o for o in prior['observations'] if o['variant']==r['variant'] and o['cohort']==r['cohort'] and o['capture_sha256']==capture]
        assert len(matches)==1;selected_observations.append(matches[0])
    candidate={**prior,'observations':selected_observations};joins={v:join(prior,candidate,v,v,contract,contract) for v in variant_ids}
    assert all(j['matched']==j['candidate_total'] and j['same_captures']==j['matched'] and j['scientific_changes']==0 for j in joins.values())
    old_keys={(o['cohort'],o['physical_sha256'],o['occurrence']) for o in prior['observations']};new_keys={(o['cohort'],o['physical_sha256'],o['occurrence']) for o in selected_observations}
    context={'schema_version':1,'references':[{'variant':v['id'],'label':v['label'],'run_id':prior['id']+'/'+v['id'],'projection_sha256':source} for v in study['variants']],
        'targets':[{'id':'native-target-landing','label':'Native verified target landing','scope':'mission','metric':'verified_target_landing','unit':'boolean','operator':'eq','threshold':1.,'observed':None,'cohort':None,'source_href':'http://starship:8000/'+old.name+'/source-bindings.json','source_sha256':sha(old/'source-bindings.json'),'method':'Recorded physical/mission/integrity/complete-replay tuple; no post-contact velocity substitution'}],
        'earlier':{'label':'Pinned prior presentation edition','projection_sha256':source,'source_href':'http://starship:8000/'+old.name+'/report.html','baseline_total':len(old_keys),'candidate_total':len(new_keys),'matched':len(old_keys&new_keys),'baseline_only':len(old_keys-new_keys),'candidate_only':len(new_keys-old_keys),'same_captures_verified':True,'scientific_changes':0},'single_preview_suffix':'-single.svg'}
    if a.kind=='timing' and not a.pilot:context['targets'].append({'id':'primary-qualification','label':'One-sided 95% lower reliability bound at least 99%','scope':'cohort','metric':'frozen_reliability_lower_bound','unit':'fraction','operator':'gte','threshold':.99,'observed':.05**(1/300),'cohort':'primary','source_href':'http://starship:8000/timing-reference/loop-20261024T185303Z/analysis.json','source_sha256':sha(TIMING/'analysis.json'),'method':'Frozen 300-original source analysis; never recomputed from a selection'})
    tag=a.kind+('-pilot' if a.pilot else '-full');spec={'title':('Timing reference' if a.kind=='timing' else 'Learned ignition')+' · comparisons and mission inspection','subtitle':'Mission → comparison → native targets → paired flight inspection','summary':'Retained complete evidence with explicit comparisons and portable inspection.','rows':rows,'grouped':True,'focus_flat':True,'max_bytes':9*1024**3,'study':study,'review_context':context}
    write_new(root/(tag+'-spec.json'),spec);write_new(root/(tag+'-source-join.json'),{'contract_sha256':contract,'earlier_projection_sha256':source,'variants':joins})
    native.render_report(json.dumps(spec,allow_nan=False),str(out))
    declarations=[TIMING/'analysis.json',TIMING/'reference-freeze.json',TIMING/'benchmark/result.json'] if a.kind=='timing' else [RL_REPORT/'report-input.json']
    write_new(out/'source-declarations.json',[{'path':str(p),'sha256':sha(p)} for p in declarations])
    write_new(out/'source-bindings.json',{'schema_version':1,'study':study['id'],'renderer_sha256':sha(__file__),'native_library_sha256':sha(native.__file__),'spec_sha256':sha(root/(tag+'-spec.json')),'native_transitions':0,'bindings':bindings})
    write_new(root/(tag+'-render.json'),{'output':str(out),'captures':len(rows),'original_occurrences':len(new_keys),'native_transitions':0,'bytes':sum(p.stat().st_size for p in out.iterdir() if p.is_file())})
    print(json.dumps({'output':str(out),'flights':len(rows),'originals':len(new_keys)}),flush=True)


if __name__=='__main__':main()
