"""Check every published UI/data byte and explicit raw-capture archive link."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
from urllib.parse import quote,urljoin,unquote,urlparse
from urllib.request import urlopen

def sha(path):
    value=hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b''):value.update(chunk)
    return value.hexdigest()


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('target',type=Path);p.add_argument('receipt',type=Path);p.add_argument('--archives',action='store_true');p.add_argument('--ui',default='ui-publish');p.add_argument('--publication',default='publication-ui.json');a=p.parse_args();target=a.target.resolve();export=json.loads((target/'export-receipt.json').read_text());checks={}
    for index,name in enumerate(export['data_bundles']):
        r=json.loads((target/f'publication-data-{index:03}.json').read_text());base=r['url'];directory=Path(name)
        for path in directory.rglob('*'):
            if path.is_file():checks[urljoin(base,quote(str(path.relative_to(directory)),safe='/'))]=path
    r=json.loads((target/a.publication).read_text());base=urljoin(r['url'],'./');ui=target/a.ui
    for path in ui.rglob('*'):
        if path.is_file():checks[urljoin(base,quote(str(path.relative_to(ui)),safe='/'))]=path
    published=len(checks)
    if a.archives:
        source=Path(export['source']);outputs=next(parent for parent in source.parents if parent.name=='outputs')
        for path in source.glob('flight-*-evidence.json'):
            e=json.loads(path.read_text());capture=(source/unquote(urlparse(e['source_evaluation_href']).path)).resolve();assert sha(capture)==e['source_evaluation_sha256'];checks['http://starship:8000/'+quote(str(capture.relative_to(outputs)),safe='/')]=capture
        for item in json.loads((source/'source-declarations.json').read_text()):
            path=Path(item['path']);assert sha(path)==item['sha256'];checks['http://starship:8000/'+quote(str(path.relative_to(outputs)),safe='/')]=path
    def check(item):
        url,path=item;expected=sha(path);value=hashlib.sha256();size=0
        with urlopen(url,timeout=60) as response:
            assert response.status==200
            for chunk in iter(lambda:response.read(1024*1024),b''):size+=len(chunk);value.update(chunk)
        assert size==path.stat().st_size and value.hexdigest()==expected,url
        return {'url':url,'path':str(path),'bytes':size,'sha256':expected,'kind':'archived-source' if ':8000/' in url else 'published-ui-data'}
    with ThreadPoolExecutor(max_workers=6) as executor:verified=list(executor.map(check,sorted(checks.items())))
    value={'passed':True,'published_files':published,'archived_files':len(checks)-published,'total_bytes':sum(c['bytes'] for c in verified),'checks':verified}
    with a.receipt.open('x') as stream:json.dump(value,stream,indent=2);stream.write('\n')
    print(json.dumps({k:v for k,v in value.items() if k!='checks'}),flush=True)


if __name__=='__main__':main()
