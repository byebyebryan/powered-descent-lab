"""Close this allocation with immutable evidence and foreign-work checks."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time


def sha(path):
    value=hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b''):value.update(chunk)
    return value.hexdigest()


def command(*args):return subprocess.check_output(['rtk','proxy',*args],text=True)


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('root',type=Path);args=parser.parse_args()
    repo=Path.cwd();root=args.root.resolve();release=root/'release-b';read=lambda p:json.loads(p.read_text())
    baseline=read(root/'workspace-baseline.json')
    owned={'pd-report/src/lib.rs','pd-report/src/rich_template.rs','pd-report/src/paired_model.js',
        'pd-report/src/paired_review.js','pd-report/src/detail_view.rs','pd-report/src/detail_navigation.js',
        'examples/coast-ignite-policy/src/report.rs','docs/report_presentations.md'}
    untouched=0
    for entry in baseline['files']:
        if entry['path'] in owned:continue
        assert sha(repo/entry['path'])==entry['sha256'],entry['path'];untouched+=1
    old_reports=0
    for report in baseline['reports']:
        for entry in report['files']:
            assert sha(Path(report['root'])/entry['path'])==entry['sha256'],entry['path'];old_reports+=1
    assert command('git','status','--short')==baseline['status'],'Foreign Git state changed or owned edits remain uncommitted'
    frozen=read(root/'source-epoch-b.json');production=0
    for entry in frozen['files']:
        if entry['path'].startswith(('pd-','examples/')):
            assert sha(repo/entry['path'])==entry['sha256'],entry['path'];production+=1
    for study in ['timing','rl']:
        assert read(release/(study+'-audit.json'))['passed']
        assert read(release/(study+'-full-render.json'))['native_transitions']==0
    for kind in ['timing','rl']:
        for viewport in ['desktop','mobile']:
            receipt=read(release/'browser-accepted'/f'{kind}-{viewport}.json')
            assert receipt['all_occurrences_browsed'] and receipt['fixed_claims'] and receipt['context_roundtrip'] and not receipt['overflow']
    delivery=read(release/'http-delivery.json');assert delivery['passed']
    assert (release/'developer-gate.log').read_text().endswith('Completed 19 planner development checks.\n')
    publication=read(release/'publication-receipt.json');assert publication['delivery']['state']=='verified' and publication['server']['reused'] and publication['server']['pid']==37689
    assert read(release/'publication-delivery.json')['passed']
    for receipt in read(release/'browser-accepted/publication.json'):assert not receipt['overflow']
    services=command('ss','-ltnp')
    assert any(':8000 ' in line and 'pid=37637,' in line for line in services.splitlines())
    assert any(':8787 ' in line and 'pid=37689,' in line for line in services.splitlines())
    artifact_roots=[root,*sorted((repo/'outputs').glob('*20261024T212129Z'))]
    sizes={str(p.relative_to(repo)):sum(f.stat().st_size for f in p.rglob('*') if f.is_file()) for p in artifact_roots}
    new_bytes=sum(sizes.values());free=shutil.disk_usage(repo).free;now=time.time()
    assert new_bytes<baseline['max_new_bytes'] and free>baseline['minimum_free_bytes'] and now<baseline['deadline']
    proofs=[p for p in release.rglob('*') if p.is_file() and p.suffix in {'.json','.mjs','.log'}]
    result={'passed':True,'head':command('git','rev-parse','HEAD').strip(),'initial_head':baseline['head'],
        'original_git_visible_files':len(baseline['files']),'unchanged_unowned_files':untouched,
        'allowed_owned_original_files':sorted(owned),'unchanged_old_report_files':old_reports,
        'foreign_git_status_preserved':True,'frozen_production_files':production,
        'library_sha256':sha(root/'native-b/pd_coast_native.so'),'source_epoch_sha256':sha(root/'source-epoch-b.json'),
        'presentation_native_transitions':0,'training_updates':0,'server_lifecycle_changes':0,
        'server_pids':{'8000':37637,'8787':37689},'delivery_files':delivery['files'],
        'artifact_bytes':sizes,'total_new_bytes':new_bytes,'maximum_new_bytes':baseline['max_new_bytes'],
        'free_bytes':free,'minimum_free_bytes':baseline['minimum_free_bytes'],'elapsed_seconds':now-baseline['start_unix'],
        'remaining_allocation_seconds':baseline['deadline']-now,'published_url':publication['url'],
        'proof_files':[{'path':str(p.relative_to(root)),'sha256':sha(p)} for p in sorted(proofs)]}
    with (release/'closure.json').open('x') as stream:json.dump(result,stream,indent=2);stream.write('\n')
    print(json.dumps({k:v for k,v in result.items() if k not in {'proof_files','artifact_bytes','allowed_owned_original_files'}},indent=2))


if __name__=='__main__':main()
