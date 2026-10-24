"""Losslessly pack a complete maintained rich edition; no evidence execution."""
import argparse
import hashlib
import json
from pathlib import Path
from lossless import compressed, html_tokens, pack, unpack

sha=lambda b:hashlib.sha256(b).hexdigest()


def split_html(text):
    marker='const reportData = ';start=text.index(marker)+len(marker)
    _,end=json.JSONDecoder().raw_decode(text,start)
    return text[:start],text[start:end],text[end:]


def build(source,target):
    target.mkdir(parents=True,exist_ok=False);data=target/'data';data.mkdir()
    lines=[];ids={};index={};receipts=[];chunks=[];pool={};cases={};previews={}
    def intern(text):
        result=[]
        for chunk in html_tokens(text):
            if chunk not in ids:ids[chunk]=len(lines);lines.append(chunk)
            result.append(ids[chunk])
        return result
    def flush():
        nonlocal pool,cases,previews
        if not cases:return
        number=len(chunks);name=f'data/flights-{number:03}.json.gz';preview=f'data/previews-{number:03}.json.gz'
        blob=compressed({'vectors':pool,'cases':cases});small=compressed(previews)
        assert len(blob)<=8*1024**2,(name,len(blob))
        (target/name).write_bytes(blob);(target/preview).write_bytes(small)
        for key in cases:index[key]={'pack':name,'previews':preview}
        chunks.append({'path':name,'bytes':len(blob),'cases':len(cases),'sha256':sha(blob)})
        pool={};cases={};previews={}
    files=sorted(source.glob('flight-*.html'),key=lambda p:int(p.stem.split('-')[1]))
    for ordinal,path in enumerate(files):
        key=path.stem.split('-')[1];text=path.read_text();before,payload,after=split_html(text)
        record={'before':intern(before),'after':intern(after),'payload':pack(payload,pool),
            'evidence':json.loads((source/f'flight-{key}-evidence.json').read_text()),
            'proof':json.loads((source/f'flight-{key}-proof.json').read_text())}
        rebuilt=''.join(lines[i] for i in record['before'])+unpack(record['payload'],pool)+''.join(lines[i] for i in record['after'])
        assert rebuilt==text,path
        receipts.append({'case':key,'html_sha256':sha(text.encode()),'payload_sha256':sha(payload.encode())})
        cases[key]=record;previews[key]=(source/f'flight-{key}.svg').read_text()
        if len(cases)==24:flush()
        if (ordinal+1)%240==0:print(json.dumps({'packed':ordinal+1,'total':len(files)}),flush=True)
    flush();(data/'html-lines.json.gz').write_bytes(compressed(lines))
    (data/'index.json').write_text(json.dumps({'schema_version':1,'cases':index},separators=(',',':')))
    for name in ['navigation.json','report-input.json','study.json']:
        (data/(name+'.gz')).write_bytes(compressed(json.loads((source/name).read_text())))
    (data/'report.html.gz').write_bytes(__import__('gzip').compress((source/'report.html').read_bytes(),6,mtime=0))
    result={'passed':True,'source':str(source),'rich_details':len(files),'chunks':chunks,
        'files':sum(p.is_file() for p in target.rglob('*')),'bytes':sum(p.stat().st_size for p in target.rglob('*') if p.is_file()),
        'roundtrip_html_exact':True,'native_transitions':0,'records':receipts}
    with (target/'packing-receipt.json').open('x') as stream:json.dump(result,stream,indent=2);stream.write('\n')
    print(json.dumps({k:v for k,v in result.items() if k not in {'records','chunks'}}),flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('source',type=Path);p.add_argument('target',type=Path);a=p.parse_args();build(a.source.resolve(),a.target.resolve())
