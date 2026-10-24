"""Independent read-only inventory, proof, projection and scientific-value audit."""
import argparse
import hashlib
import json
import re
from collections import Counter,defaultdict
from pathlib import Path
from urllib.parse import unquote,urlparse

read=lambda p:json.loads(Path(p).read_text())
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()


def report_data(path):
    text=Path(path).read_text();marker='const reportData = ';start=text.index(marker)+len(marker)
    return json.JSONDecoder().raw_decode(text[start:])[0]


def audit(output,spec_path):
    projection=read(output/'study.json');review=read(output/'report-input.json');navigation=read(output/'navigation.json');spec=read(spec_path)
    bindings=read(output/'source-bindings.json');assert bindings['spec_sha256']==sha(spec_path)
    baseline=read(spec_path.parent/'workspace-baseline.json');original_expected={}
    for report in baseline['reports']:
        old=Path(report['root']);identities={f['path']:f['sha256'] for f in report['files']}
        for name,digest in identities.items():
            if not name.endswith('-evidence.json'):continue
            assert sha(old/name)==digest
            record=read(old/name);proof_name=name.replace('-evidence.json','-proof.json')
            assert sha(old/proof_name)==identities[proof_name]
            proof=read(old/proof_name);capture=(old/unquote(urlparse(record['source_evaluation_href']).path)).resolve().parent
            original_expected[str(capture)]=(record['source_evaluation_sha256'],proof['decisions_sha256'])
    assert len(review['missions'])==sum(c['originals'] for c in projection['cohorts'])
    assert navigation['study']==projection
    assert {k:v for k,v in navigation.items() if k!='study'}==review
    assert len(projection['observations'])==len(spec['rows'])==len(bindings['bindings'])
    assert bindings['native_transitions']==0
    for declaration in read(output/'source-declarations.json'):assert sha(declaration['path'])==declaration['sha256']
    observed={(o['mission'],o['variant']):o for o in projection['observations']};lookup={}
    for m in review['missions']:
        for attempt in m['attempts']:
            i=attempt['attempt_index'];o=observed[(m['id'],projection['variants'][attempt['approach']]['id'])]
            lookup[i]=(m,attempt,o)
    sources=[];speed_domains=defaultdict(set)
    for index,row in enumerate(spec['rows']):
        folder=Path(row['capture']);evaluation=read(folder/'evaluation.json');proof=read(folder/'proof.json');seal=read(folder/'seal.json');info=read(folder/'decisions.json')
        assert sha(folder/'evaluation.json')==proof['capture_sha256']==seal['evaluation_sha256']
        assert sha(folder/'decisions.json')==proof['decisions_sha256']
        assert original_expected[str(folder)]==(proof['capture_sha256'],proof['decisions_sha256'])
        assert evaluation['integrity_passed'] and evaluation['replay']['complete_episode_verified'] and evaluation['replay']['physical_prefix_verified']
        assert all(proof[k] for k in ['physical_replay','controller_reconstruction','planner_reconstruction'])
        m,attempt,o=lookup[index];assert m['physical_sha256']==evaluation['physical_sha256']==o['physical_sha256']
        assert row['cohort']==o['cohort'] and row['variant']==o['variant'] and m['occurrence']==row.get('occurrence',0)
        assert evaluation['verified_target_landing']==o['verified_landing']==(attempt['outcome']=='landed')
        final=evaluation['execution']['episode']['final_state'];initial=evaluation['scenario']['vehicle']['initial_fuel_kg']
        assert o['values']['fuel_used_kg']==initial-final['fuel_kg']
        contact=evaluation['execution']['episode']['incoming_contact']
        if contact:
            v=contact['state']['velocity_mps'];expected=(v['x']**2+v['y']**2)**.5
            assert abs(o['values']['contact_speed_mps']-expected)<1e-12
        else:assert 'contact_speed_mps' not in o['values']
        e=read(output/f'flight-{index}-evidence.json');p=read(output/f'flight-{index}-proof.json')
        assert p==proof and e['source_evaluation_sha256']==proof['capture_sha256']
        assert e['incoming_contact']==contact and e['final_state']==final and e['policy_decisions']==info
        assert e['physical_sha256']==o['physical_sha256'] and e['manifest']==evaluation['execution']['episode']['run']['manifest']
        # Every serialized sample/command/event displayed by the new rich view
        # must be an exact projection of the retained native values.
        data=report_data(output/f'flight-{index}.html');samples=evaluation['execution']['episode']['run']['samples']
        assert len(data['samples'])==len(samples)
        updates=evaluation['execution']['controller_updates'];update_index=0
        for view,source in zip(data['samples'],samples):
            ob=source['observation'];assert view['physicsStep']==source['physics_step'] and view['simTimeS']==source['sim_time_s']
            for key,expected in [('xM',ob['position_m']['x']),('yM',ob['position_m']['y']),('vxMps',ob['velocity_mps']['x']),('vyMps',ob['velocity_mps']['y']),('fuelKg',ob['fuel_kg']),('attitudeRad',ob['attitude_rad'])]:assert view[key]==expected
            while update_index+1<len(updates) and updates[update_index+1]['physics_step']<=source['physics_step']:update_index+=1
            command=updates[update_index]['frame']['command'] if updates and updates[update_index]['physics_step']<=source['physics_step'] else source['held_command']
            assert view['throttleFrac']==command['throttle_frac'] and view['targetAttitudeRad']==command['target_attitude_rad']
        html=(output/f'flight-{index}.html').read_text();match=re.search(r'id="spatial-color-domains">(.*?)</script>',html)
        assert match;domain=json.loads(match.group(1));speed_domains[o['physical_sha256']].add(json.dumps(domain,sort_keys=True))
        sources.append({'capture':str(folder),'evaluation_sha256':sha(folder/'evaluation.json'),'decisions_sha256':sha(folder/'decisions.json')})
    assert all(len(v)==1 for v in speed_domains.values())
    cohorts={c['id']:c for c in projection['cohorts']};matrix={};fuel_eligible={}
    by=defaultdict(dict)
    for o in projection['observations']:by[(o['cohort'],o['physical_sha256'],o['occurrence'])][o['variant']]=o
    for contrast in projection['contrasts']:
        for cohort in cohorts:
            counts=Counter();eligible=0;deltas=[];lost=[]
            for (role,_,_),records in by.items():
                if role!=cohort:continue
                a,b=(records.get(contrast[k]) for k in ['baseline','candidate'])
                if a is None or b is None:counts['unpaired']+=1;continue
                label=('preserved' if b['verified_landing'] else 'lost') if a['verified_landing'] else ('gained' if b['verified_landing'] else 'missed')
                counts[label]+=1
                if label=='lost':lost.extend(m['mission_index'] for m in review['missions'] if m['id']==a['mission'])
                if a['verified_landing'] and b['verified_landing']:
                    eligible+=1;deltas.append(b['values']['fuel_used_kg']-a['values']['fuel_used_kg'])
            assert sum(counts.values())==cohorts[cohort]['originals']
            key=cohort+'/'+contrast['id'];matrix[key]={'counts':dict(counts),'lost_originals':lost};fuel_eligible[key]=eligible
            effect=next((e for e in projection['effects'] if e['cohort']==cohort and e['contrast']==contrast['id'] and e['metric']=='fuel_used_kg'),None)
            if effect:assert effect['pairs']==eligible and abs(effect['estimate']-sum(deltas)/len(deltas))<1e-8
    if projection['id'].startswith('timing'):
        assert cohorts['primary']['originals']==300
        assert matrix['primary/expanded']['counts']=={'preserved':300} and fuel_eligible['primary/expanded']==300
        assert matrix['regressions/expanded']['counts']=={'unpaired':21}
        assert all(p['originals']==12 and p['measured_repeats']==7 and len(p['values'])==84 for p in projection['performance'])
        declarations=read(output/'source-declarations.json')
        analysis=read(next(d['path'] for d in declarations if d['path'].endswith('/analysis.json')))
        benchmark=read(next(d['path'] for d in declarations if d['path'].endswith('/benchmark/result.json')))
        for p in projection['performance']:
            assert p['values']==[r['measurements']['selection_wall_ns'][0]/1e6 for r in benchmark['rows'] if r['mode']==p['variant'] and r['role']=='measured']
        for e in projection['effects']:
            contrast=next(c for c in projection['contrasts'] if c['id']==e['contrast'])
            source=analysis['populations']['primary']['paired'][contrast['baseline']+'__'+contrast['candidate']]
            assert (e['estimate'],e['low'],e['high'])==(-source['mean_saving_kg'],-source['mean_saving_kg_ci95'][1],-source['mean_saving_kg_ci95'][0])
    else:
        assert matrix['confirmation/heuristic-rl']['counts']=={'preserved':443,'gained':146,'lost':3,'missed':8}
        assert matrix['confirmation/heuristic-rl']['lost_originals']==[355,436,514]
        assert fuel_eligible['confirmation/heuristic-rl']==443
    return {'passed':True,'edition':str(output),'rich_flights':len(spec['rows']),'original_occurrences':len(review['missions']),
        'matrices':matrix,'fuel_eligible':fuel_eligible,'shared_speed_domains':len(speed_domains),'native_transitions':0,'source_bindings':sources}


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('edition',type=Path);p.add_argument('spec',type=Path);p.add_argument('receipt',type=Path);a=p.parse_args()
    result=audit(a.edition.resolve(),a.spec.resolve())
    with a.receipt.open('x') as f:json.dump(result,f,indent=2);f.write('\n')
    print(json.dumps({k:v for k,v in result.items() if k!='source_bindings'},indent=2))
