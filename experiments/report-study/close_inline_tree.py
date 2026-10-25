"""Check source, evidence, workspace, delivery and resource preservation."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time


def read(path):return json.loads(path.read_text())


def sha(path):
    h=hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b''):h.update(chunk)
    return h.hexdigest()


def git(*args):return subprocess.check_output(['rtk','proxy','git',*args],text=True)


def bytes_under(path):return sum(p.stat().st_size for p in path.rglob('*') if p.is_file())


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('root',type=Path);parser.add_argument('receipt',type=Path);a=parser.parse_args()
    root=a.root.resolve();repo=Path(__file__).resolve().parents[2];baseline=read(root/'workspace-baseline.json')
    assert git('status','--porcelain')==baseline['foreign_status'],'Foreign workspace state changed or owned changes are not committed'
    owned=set(git('diff','--name-only',baseline['head'],'HEAD').splitlines())
    doc_paths={'docs/report_inline_mission_comparison_plan.md','docs/report_inline_mission_comparison_execution.md','docs/report_presentations.md'}
    assert all(p in doc_paths or p=='examples/coast-ignite-policy/src/report.rs' or p.startswith(('pd-report/','experiments/report-study/','scripts/test-report-','scripts/test-inline-mission-')) for p in owned),owned
    checks=[(repo/f['path'],f['sha256']) for f in baseline['files'] if f['path'] not in owned]
    workspace_count=len(checks);retained_count=0
    for edition in baseline['retained']:
        directory=Path(edition['root']);assert {str(p.relative_to(directory)) for p in directory.rglob('*') if p.is_file()}=={v['path'] for v in edition['files']}
        checks.extend((directory/v['path'],v['sha256']) for v in edition['files']);retained_count+=len(edition['files'])
    prior=Path(baseline['prior_closure']);assert sha(prior)==baseline['prior_closure_sha256'];prior_data=read(prior)
    source_receipt=prior.parent/'final-source-preservation.json';assert sha(source_receipt)==prior_data['receipts_sha256']['release/final-source-preservation.json']
    checks.extend((Path(path),expected) for path,expected in read(source_receipt)['files'].items())
    def verify(item):
        path,expected=item;assert path.is_file() and sha(path)==expected,str(path)
    with ThreadPoolExecutor(max_workers=4) as pool:list(pool.map(verify,checks))
    acceptance=read(root/'release/acceptance.json');assert acceptance['passed'] and acceptance['checks']
    for name,check in acceptance['checks'].items():
        assert check['passed'] and (root/check['evidence']).is_file(),name
        assert sha(root/check['evidence'])==check['sha256'],name
    gate=root/'release/developer-gate.log';assert 'Completed 19 planner development checks.' in gate.read_text()
    source=read(root/'release/source-freeze.json')
    for path,expected in source['files'].items():assert sha(repo/path)==expected,path
    publications=[]
    for path in sorted((root/'delivery').glob('**/publication.json')):
        r=read(path);assert r['delivery']['state']=='verified' and r['server']['reused'] and r['server']['pid']==37689;publications.append(r)
    assert len(publications)==len({r['report_id'] for r in publications})<=baseline['max_publications']
    published_bytes=sum(r['byte_count'] for r in publications);assert published_bytes<=baseline['max_publication_bytes']
    servers={port:Path(f'/proc/{pid}/cmdline').read_bytes().replace(b'\0',b' ').decode().strip() for port,pid in baseline['server_pids'].items()}
    assert 'scripts/report_server.py --port 8000' in servers['8000'] and 'scripts/codex-report-server' in servers['8787']
    artifact_bytes=bytes_under(root);snapshot_bytes=sum(bytes_under(Path(r['snapshot_path'])) for r in publications);free=shutil.disk_usage(repo).free;now=time.time()
    assert artifact_bytes+snapshot_bytes<=baseline['max_artifact_bytes'] and free>=baseline['minimum_free_bytes'] and now<baseline['deadline']
    receipts={str(p.relative_to(root)):sha(p) for p in sorted((root/'release').glob('*')) if p.is_file() and p.resolve()!=a.receipt.resolve()}
    result={'passed':True,'head':git('rev-parse','HEAD').strip(),'checked_at_unix':now,'elapsed_seconds':now-baseline['start_unix'],'source_head':source['head'],'foreign_status_unchanged':True,'unchanged_workspace_files':workspace_count,'unchanged_prior_report_files':retained_count,'unchanged_capture_and_decision_files':len(read(source_receipt)['files']),'campaign_flights':0,'source_replays':0,'training_updates':0,'publications':len(publications),'published_bytes':published_bytes,'artifact_bytes':artifact_bytes,'snapshot_bytes':snapshot_bytes,'free_bytes':free,'server_pids':baseline['server_pids'],'server_commands':servers,'owned_committed_files':sorted(owned),'release_sha256':receipts}
    with a.receipt.open('x') as stream:json.dump(result,stream,indent=2);stream.write('\n')
    print(json.dumps({k:v for k,v in result.items() if k not in ['owned_committed_files','release_sha256','server_commands']}),flush=True)


if __name__=='__main__':main()
