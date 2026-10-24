"""Explicit saved-run join; identities and unmatched records remain visible."""
import argparse
import json
from pathlib import Path


def join(baseline,candidate,baseline_variant,candidate_variant,baseline_contract,candidate_contract):
    if not baseline_contract or baseline_contract!=candidate_contract:raise ValueError('Incompatible physical/evaluation contract')
    if baseline['metrics']!=candidate['metrics']:raise ValueError('Incompatible metric semantics')
    def index(study,variant):
        result={}
        for o in study['observations']:
            if o['variant']!=variant:continue
            key=(o['cohort'],o['physical_sha256'],o['occurrence'])
            if key in result:raise ValueError('Ambiguous repeated occurrence')
            result[key]=o
        return result
    left,right=index(baseline,baseline_variant),index(candidate,candidate_variant);common=left.keys()&right.keys();changed=[];same=[]
    for key in sorted(common):
        a,b=left[key],right[key]
        if (a['outcome'],a['verified_landing'],a['values'])!=(b['outcome'],b['verified_landing'],b['values']):changed.append(key)
        if a['capture_sha256']==b['capture_sha256']:same.append(key)
    return {'baseline_total':len(left),'candidate_total':len(right),'matched':len(common),'baseline_only':len(left.keys()-right.keys()),'candidate_only':len(right.keys()-left.keys()),'scientific_changes':len(changed),'same_captures':len(same),'compatible_contract':baseline_contract,'changed_originals':[list(k) for k in changed]}


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('baseline',type=Path);p.add_argument('candidate',type=Path);p.add_argument('--baseline-variant',required=True);p.add_argument('--candidate-variant',required=True);p.add_argument('--baseline-contract',required=True);p.add_argument('--candidate-contract',required=True);p.add_argument('--receipt',type=Path,required=True);a=p.parse_args()
    r=join(json.loads(a.baseline.read_text()),json.loads(a.candidate.read_text()),a.baseline_variant,a.candidate_variant,a.baseline_contract,a.candidate_contract)
    with a.receipt.open('x') as stream:json.dump(r,stream,indent=2)
