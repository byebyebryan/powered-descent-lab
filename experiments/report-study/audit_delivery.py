"""Verify served bytes for complete editions and their linked source records."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
from urllib.parse import quote, unquote, urlparse
from urllib.request import urlopen


def digest(path):
    value=hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b''):value.update(chunk)
    return value.hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('outputs',type=Path)
    parser.add_argument('receipt',type=Path)
    parser.add_argument('editions',type=Path,nargs='+')
    args=parser.parse_args();root=args.outputs.resolve();files=set()
    for edition in args.editions:
        edition=edition.resolve()
        files.update(p for p in edition.rglob('*') if p.is_file())
        for record in edition.glob('flight-*-evidence.json'):
            evidence=json.loads(record.read_text())
            source=(edition/unquote(urlparse(evidence['source_evaluation_href']).path)).resolve()
            assert digest(source)==evidence['source_evaluation_sha256']
            files.add(source)
        for declaration in json.loads((edition/'source-declarations.json').read_text()):
            source=Path(declaration['path']).resolve();assert digest(source)==declaration['sha256'];files.add(source)
    def check(path):
        relative=str(path.relative_to(root));url='http://starship:8000/'+quote(relative,safe='/')
        expected=digest(path);size=0;actual=hashlib.sha256()
        with urlopen(url,timeout=60) as response:
            assert response.status==200
            for chunk in iter(lambda:response.read(1024*1024),b''):size+=len(chunk);actual.update(chunk)
        assert size==path.stat().st_size and actual.hexdigest()==expected,url
        return {'path':relative,'url':url,'bytes':size,'sha256':expected}
    with ThreadPoolExecutor(max_workers=6) as workers:checked=list(workers.map(check,sorted(files)))
    result={'passed':True,'files':len(checked),'bytes':sum(c['bytes'] for c in checked),'checks':checked}
    with args.receipt.open('x') as stream:json.dump(result,stream,indent=2);stream.write('\n')
    print(json.dumps({k:v for k,v in result.items() if k!='checks'}))


if __name__=='__main__':main()
