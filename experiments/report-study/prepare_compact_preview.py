"""Create a native layout preview over retained evidence, without copying flights."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
from urllib.parse import urljoin


def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('source',type=Path);p.add_argument('packed',type=Path)
    p.add_argument('spec',type=Path);p.add_argument('target',type=Path)
    p.add_argument('--source-url',required=True)
    a=p.parse_args();repo=Path(__file__).resolve().parents[2]
    spec=json.loads(a.spec.read_text());patches=[];sources={}
    for item in spec['patches']:
        if 'before_text' in item:
            before,after=item['before_text'],item['after_text']
        else:
            before_path=Path(item['before_path']);before=before_path.read_text()
            paths=[repo/name for name in item['after_paths']]
            after='\n'.join(path.read_text() for path in paths)
            for path in [before_path,*paths]:sources[str(path)] = sha(path)
        patches.append((before,after,item['views'],item['id']))
    # Paired JSON is display metadata; only its links are rebased. No native
    # scientific flight payload is read, serialized or rewritten here.
    def projection(value):
        if isinstance(value,dict):
            for key,entry in value.items():
                if key in ('href','preview','evidence_href') and isinstance(entry,str) and not entry.startswith('#'):
                    value[key]=urljoin(a.source_url,entry)
                else:projection(entry)
        elif isinstance(value,list):
            for entry in value:projection(entry)
        return value
    import re
    a.target.mkdir()
    for name,view in [('report.html','study'),('compare.html','pair')]:
        text=gzip.decompress((a.packed/'data'/(name+'.gz')).read_bytes()).decode()
        for before,after,views,key in patches:
            assert text.count(before)==int(view in views),(key,view)
            text=text.replace(before,after)
        pattern=r'(<script type="application/json" id="paired-review-data">)([\s\S]*?)(</script>)'
        text=re.sub(pattern,lambda m:m[1]+json.dumps(projection(json.loads(m[2]))).replace('<','\\u003c')+m[3],text)
        text=re.sub(r'href="([^"]+)"',lambda m:m[0] if m[1].startswith('#') or '${' in m[1] or m[1]=='compare.html' else 'href="'+urljoin(a.source_url,m[1])+'"',text)
        (a.target/name).open('x').write(text)
    for name in ['report-input.json','navigation.json','study.json','comparison-review.json','source-bindings.json','source-declarations.json']:
        value=json.loads((a.source/name).read_text())
        (a.target/name).open('x').write(json.dumps(projection(value))+'\n')
    result={'passed':True,'kind':'native layout preview','native_transitions':0,'source':str(a.source.resolve()),'source_url':a.source_url,'module_sha256':sources,'files':{p.name:sha(p) for p in a.target.iterdir()}}
    (a.target/'preview-receipt.json').open('x').write(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'passed':True,'target':str(a.target),'files':len(result['files']),'native_transitions':0}))


if __name__=='__main__':main()
