"""Publish bounded immutable data bundles, then the complete study UI."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
from urllib.parse import urlparse


def read(path):return json.loads(path.read_text())


def publish(source,title,receipt,entrypoint='index.html'):
    if receipt.exists():return read(receipt)
    result=subprocess.run(['rtk','proxy','codex-plus-report','--json','publish','--source',str(source),'--title',title,'--project','powered-descent-lab','--entrypoint',entrypoint,'--check-delivery'],text=True,capture_output=True)
    if result.returncode:raise RuntimeError(result.stderr or result.stdout)
    value=json.loads(result.stdout)
    assert value['delivery']['state']=='verified' and value['server']['reused'] and value['server']['pid']==37689
    with receipt.open('x') as stream:json.dump(value,stream,indent=2);stream.write('\n')
    return value


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('target',type=Path);p.add_argument('--title',required=True);a=p.parse_args();target=a.target.resolve();export=read(target/'export-receipt.json');manifest=read(Path(export['ui'])/'asset-manifest.json');published={};receipts=[]
    for ordinal,name in enumerate(export['data_bundles']):
        directory=Path(name);r=publish(directory,a.title+f' · immutable flight data {ordinal+1}/{len(export["data_bundles"])}',target/f'publication-data-{ordinal:03}.json');receipts.append(r)
        for path in (directory/'data').iterdir():published['data/'+path.name]=urlparse(r['url']).path.rstrip('/')+'/data/'+path.name
        print(json.dumps({'published_data':ordinal+1,'total':len(export['data_bundles']),'bytes':r['byte_count']}),flush=True)
    assert published.keys()==manifest['assets'].keys()
    ui=target/'ui-publish'
    if not ui.exists():
        ui.mkdir()
        for path in Path(export['ui']).iterdir():
            if path.name=='asset-manifest.json':continue
            if path.is_dir():shutil.copytree(path,ui/path.name)
            else:shutil.copyfile(path,ui/path.name)
        with (ui/'asset-manifest.json').open('x') as stream:json.dump({**manifest,'assets':published},stream,indent=2);stream.write('\n')
    else:assert read(ui/'asset-manifest.json')['assets']==published
    r=publish(ui,a.title,target/'publication-ui.json','report.html');receipts.append(r)
    result={'passed':True,'url':r['url'],'project_url':r['project_url'],'publications':len(receipts),'bytes':sum(r['byte_count'] for r in receipts),'receipts':[r['report_id'] for r in receipts]}
    assert result['bytes']<=1024**3 and result['publications']<=16
    with (target/'publication.json').open('x') as stream:json.dump(result,stream,indent=2);stream.write('\n')
    print(json.dumps(result),flush=True)


if __name__=='__main__':main()
