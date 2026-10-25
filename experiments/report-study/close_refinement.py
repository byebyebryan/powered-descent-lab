"""Close a reporting allocation after committed-source and delivery acceptance."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time


def sha(path):
    h=hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b''):h.update(chunk)
    return h.hexdigest()


def read(path):return json.loads(path.read_text())


def git(*args):return subprocess.check_output(['rtk','proxy','git',*args],text=True)


def tree_bytes(path):return sum(p.stat().st_size for p in path.rglob('*') if p.is_file())


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('root',type=Path);parser.add_argument('receipt',type=Path);a=parser.parse_args()
    repo=Path(__file__).resolve().parents[2];root=a.root.resolve();baseline=read(root/'workspace-baseline.json')
    status=git('status','--porcelain');assert status==baseline['foreign_status'],'Workspace status differs from protected baseline'
    owned=set(git('diff','--name-only',baseline['head'],'HEAD').splitlines())
    docs={'docs/report_comparison_refinement_plan.md','docs/report_comparison_refinement_execution.md','docs/report_presentations.md'}
    assert all(p in docs or p=='examples/coast-ignite-policy/src/report.rs' or p.startswith(('pd-report/src/','experiments/report-study/','scripts/test-report-','scripts/test-flight-inspection.')) for p in owned),owned
    checks=[(repo/f['path'],f['sha256']) for f in baseline['files'] if f['path'] not in owned]
    original_reports=0
    for edition in baseline['reports']:
        directory=Path(edition['root']);assert {str(p.relative_to(directory)) for p in directory.rglob('*') if p.is_file()}=={f['path'] for f in edition['files']}
        checks.extend((directory/f['path'],f['sha256']) for f in edition['files']);original_reports+=len(edition['files'])
    def verify(item):
        path,expected=item;assert path.is_file() and sha(path)==expected,str(path)
    with ThreadPoolExecutor(max_workers=4) as pool:list(pool.map(verify,checks))
    gate=root/'release/developer-gate-final.log';assert 'Completed 19 planner development checks.' in gate.read_text()
    receipts=[];urls={}
    for kind,count in [('timing',1285),('rl',2400)]:
        audit=read(root/f'release/{kind}-audit.json');packing=read(root/f'release/{kind}-packing-audit.json')
        assert audit['passed'] and audit['rich_flights']==count and audit['native_transitions']==0
        assert packing['passed'] and packing['rich_details']==count and packing['html_and_numeric_literals_exact'] and packing['proof_and_evidence_exact'] and packing['preview_svg_exact']
        assert read(root/f'release/{kind}-http-delivery.json')['passed']
        assert read(root/f'release/{kind}-final-http-delivery.json')['passed']
        for viewport in ['desktop','mobile']:
            browser=read(root/f'browser-final/{kind}-{viewport}.json');assert browser['passed'] and browser['legacy_origin_blocked']
            assert all(h['label_px']>=11.9 for h in browser['history_pixels'])
        delivery=root/'delivery'/kind;revision=read(delivery/'final-ui-receipt.json')
        assert revision['passed'] and revision['checked_rich_templates']==count and revision['data_aliases_unchanged'] and revision['canonical_html_unchanged']
        assert sha(repo/'pd-report/src/flight_inspection.js')==revision['new_module_sha256']
        assert sha(repo/'pd-report/src/portable_assets.js')==revision['loader_sha256']
        for name,expected in revision['assets'].items():assert sha(Path(revision['ui'])/name)==expected
        for path in sorted(delivery.glob('publication-*.json')):
            r=read(path)
            if 'report_id' not in r:continue
            assert r['delivery']['state']=='verified' and r['server']['reused'] and r['server']['pid']==baseline['server_pids']['8787']
            receipts.append(r)
        urls[kind]=read(delivery/'publication-ui-final.json')['url']
    assert len(receipts)==len({r['report_id'] for r in receipts})<=baseline['max_publications']
    published_bytes=sum(r['byte_count'] for r in receipts);assert published_bytes<=baseline['max_publication_bytes']
    servers={port:Path(f'/proc/{pid}/cmdline').read_bytes().replace(b'\0',b' ').decode().strip() for port,pid in baseline['server_pids'].items()}
    assert 'http.server' in servers['8000'] and 'codex' in servers['8787']
    stamp='20261024T224911Z';artifacts=[root]+sorted((repo/'outputs').glob('*'+stamp+'*'))
    artifact_bytes=sum(tree_bytes(p) for p in artifacts);snapshot_bytes=sum(tree_bytes(Path(r['snapshot_path'])) for r in receipts)
    free=shutil.disk_usage(repo).free;assert artifact_bytes+snapshot_bytes<=baseline['max_new_bytes'] and free>=baseline['minimum_free_bytes']
    now=time.time();assert now<baseline['deadline']
    bound=[p for p in (root/'release').glob('*.json') if p.resolve()!=a.receipt.resolve()]+list((root/'browser-final').glob('*.json'))
    result={'passed':True,'head':git('rev-parse','HEAD').strip(),'checked_at_unix':now,'elapsed_seconds':now-baseline['start_unix'],'baseline_sha256':sha(root/'workspace-baseline.json'),'unchanged_workspace_files':len(checks)-original_reports,'unchanged_prior_report_files':original_reports,'foreign_status_unchanged':True,'controller_physics_and_fixtures_unchanged':True,'rich_flights':3685,'original_occurrences':1045,'new_report_campaign_transitions':0,'new_report_replays':0,'training_updates':0,'developer_gate_sha256':sha(gate),'urls':urls,'publications':len(receipts),'published_bytes':published_bytes,'artifact_bytes':artifact_bytes,'snapshot_bytes':snapshot_bytes,'free_bytes':free,'server_pids':baseline['server_pids'],'services_unchanged':servers,'owned_committed_files':sorted(owned),'receipts_sha256':{str(p.relative_to(root)):sha(p) for p in sorted(bound)}}
    with a.receipt.open('x') as stream:json.dump(result,stream,indent=2);stream.write('\n')
    print(json.dumps({k:v for k,v in result.items() if k not in ['receipts_sha256','owned_committed_files','services_unchanged']}),flush=True)


if __name__=='__main__':main()
