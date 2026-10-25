"""Verify a new inline-tree UI and every reused published data asset."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
from urllib.parse import quote,urljoin,urlparse
from urllib.request import urlopen


def sha(path):
    h=hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b''):h.update(chunk)
    return h.hexdigest()


def read(path):return json.loads(path.read_text())


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('export',type=Path);parser.add_argument('publication',type=Path);parser.add_argument('receipt',type=Path)
    a=parser.parse_args();export=read(a.export/'export-receipt.json');publication=read(a.publication);ui=Path(export['ui']);manifest=read(ui/'asset-manifest.json')
    assert publication['delivery']['state']=='verified' and publication['server']['reused'] and publication['server']['pid']==37689
    base=urljoin(publication['url'],'./');origin=urlparse(base);checks={}
    for name,identity in export['assets'].items():
        path=ui/name;assert sha(path)==identity['sha256'] and path.stat().st_size==identity['bytes'];checks[urljoin(base,quote(name,safe='/'))]=path
    ui_count=len(checks);previous=Path(export['previous']);old_export=read(previous/'export-receipt.json');data={}
    for ordinal,directory in enumerate(old_export['data_bundles']):
        directory=Path(directory);published=read(previous/f'publication-data-{ordinal:03}.json')
        for path in (directory/'data').iterdir():
            if not path.is_file():continue
            key='data/'+path.name;assert key not in data;url=urljoin(published['url'],quote(key,safe='/'));assert urlparse(url).netloc==origin.netloc
            assert urljoin(base,manifest['assets'][key])==url;data[key]=path;checks[url]=path
    assert data.keys()==manifest['assets'].keys()

    def verify(item):
        url,path=item;expected=sha(path);h=hashlib.sha256();size=0
        with urlopen(url,timeout=60) as response:
            assert response.status==200
            for chunk in iter(lambda:response.read(1024*1024),b''):size+=len(chunk);h.update(chunk)
        assert size==path.stat().st_size and h.hexdigest()==expected,url
        return {'url':url,'path':str(path),'sha256':expected,'bytes':size}
    with ThreadPoolExecutor(max_workers=6) as pool:verified=list(pool.map(verify,sorted(checks.items())))
    result={'passed':True,'url':publication['url'],'source_head':export['source_head'],'published_ui_files':ui_count,'reused_data_files':len(data),'new_data_publications':0,'verified_bytes':sum(v['bytes'] for v in verified),'checks':verified}
    with a.receipt.open('x') as stream:json.dump(result,stream,indent=2);stream.write('\n')
    print(json.dumps({k:v for k,v in result.items() if k!='checks'}),flush=True)


if __name__=='__main__':main()
