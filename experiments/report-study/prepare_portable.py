"""Prepare owned UI/data exports for existing immutable report publication."""
import argparse
import hashlib
import html
import json
from pathlib import Path
import shutil

REPO=Path(__file__).resolve().parents[2]


def write(path,value):
    with path.open('x') as stream:stream.write(value)


def wrapper(title,code):
    return '<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>'+html.escape(title)+'</title><script src="assets/fflate-0.8.2.min.js"></script><script src="assets/portable_assets.js"></script></head><body style="font:16px/1.5 system-ui;padding:24px"><p id="loading">Loading saved evidence…</p><p><a href="report.html">Back to study and mission tree</a></p><script>(async()=>{try{'+code+'}catch(error){document.getElementById("loading").textContent="Saved view unavailable: "+error.message;document.body.dataset.loadError="true"}})();</script></body></html>'


def build(packed,target,vendor):
    target.mkdir(parents=True,exist_ok=False);ui=target/'ui-local';ui.mkdir();(ui/'assets').mkdir()
    receipt=json.loads((packed/'packing-receipt.json').read_text());assets={};bundles=[];current=None;size=0
    for path in sorted((packed/'data').iterdir()):
        if current is None or size+path.stat().st_size>90*1024**2:
            current=target/f'data-{len(bundles):03}';current.mkdir();(current/'data').mkdir();write(current/'index.html','<!doctype html><html lang="en"><meta charset="utf-8"><title>Immutable saved flight data</title><p>Supporting data for the complete mission study. Open the study entrypoint to browse.</p></html>');bundles.append(current);size=0
        rel='data/'+path.name;shutil.copyfile(path,current/rel);assets[rel]='../'+current.name+'/'+rel;size+=path.stat().st_size
    for name in ['fflate-0.8.2.min.js','plotly-basic-2.35.2.min.js','fflate-LICENSE','plotly-LICENSE']:shutil.copyfile(vendor/name,ui/'assets'/name)
    shutil.copyfile(REPO/'pd-report/src/portable_assets.js',ui/'assets/portable_assets.js')
    source=Path(receipt['source']);manifest={'schema_version':1,'source_base':'http://starship:8000/'+source.name+'/','assets':assets,'rich_details':receipt['rich_details'],'source_packing_sha256':hashlib.sha256((packed/'packing-receipt.json').read_bytes()).hexdigest()}
    write(ui/'asset-manifest.json',json.dumps(manifest,indent=2)+'\n')
    write(ui/'report.html',wrapper('Complete study and mission review',"await PdSavedAssets.page('report.html');"))
    write(ui/'compare.html',wrapper('Paired flight inspection',"await PdSavedAssets.page('compare.html');"))
    write(ui/'detail.html',wrapper('Complete saved rich flight',"const key=new URLSearchParams(location.search).get('case');if(!/^\\d+$/.test(key||''))throw Error('Unknown case');await PdSavedAssets.showDetail(key);"))
    code="const p=new URLSearchParams(location.search);let value;if(p.has('file'))value=await PdSavedAssets.json(p.get('file'));else{const r=await PdSavedAssets.record(p.get('case'));value=r.record[p.get('kind')==='proof'?'proof':'evidence'];}document.getElementById('loading').textContent='Recorded source evidence';const pre=document.createElement('pre');pre.style.cssText='white-space:pre-wrap;overflow-wrap:anywhere';pre.textContent=JSON.stringify(value,null,2);document.body.append(pre);if(value.source_evaluation_href){const m=await PdSavedAssets.manifest,a=document.createElement('a');a.href=new URL(value.source_evaluation_href,m.source_base);a.textContent='Open full native capture on the lab archive (8000)';document.body.prepend(a);}"
    write(ui/'evidence.html',wrapper('Saved source and proof evidence',code))
    write(ui/'index.html','<!doctype html><html lang="en"><meta charset="utf-8"><title>Saved study</title><a href="report.html">Open the complete study and mission tree</a><script>location.replace("report.html"+location.search+location.hash)</script></html>')
    write(ui/'provenance.json',json.dumps({k:v for k,v in receipt.items() if k not in {'records','chunks'}},indent=2))
    result={'source':str(source),'packing':str(packed),'ui':str(ui),'data_bundles':[str(p) for p in bundles],'bytes':sum(p.stat().st_size for p in target.rglob('*') if p.is_file()),'rich_details':receipt['rich_details']}
    write(target/'export-receipt.json',json.dumps(result,indent=2)+'\n');print(json.dumps(result),flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('packed',type=Path);p.add_argument('target',type=Path);p.add_argument('vendor',type=Path);a=p.parse_args();build(a.packed.resolve(),a.target.resolve(),a.vendor.resolve())
