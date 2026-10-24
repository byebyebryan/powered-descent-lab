"""Independently decode every packed rich document and check original bytes."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path

sha=lambda b:hashlib.sha256(b).hexdigest()


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('packed',type=Path);p.add_argument('receipt',type=Path);a=p.parse_args();root=a.packed.resolve()
    declared=json.loads((root/'packing-receipt.json').read_text());source=Path(declared['source']);index=json.loads((root/'data/index.json').read_text())['cases'];expected={r['case']:r for r in declared['records']};checked=[]
    for chunk in declared['chunks']:
        raw=(root/chunk['path']).read_bytes();assert sha(raw)==chunk['sha256'] and len(raw)<=8*1024**2
        data=json.loads(gzip.decompress(raw))
        for key,r in data['cases'].items():
            columns=[]
            for ident,negative in r['payload']['columns']:
                col=data['vectors'][ident].split('\n')
                if negative:col=[v if float(v)==0 else v[1:] if v.startswith('-') else '-'+v for v in col]
                columns.append(col)
            payload=r['payload']['prefix']+'['+','.join('{'+','.join(k+':'+columns[j][i] for j,k in enumerate(r['payload']['keys']))+'}' for i in range(r['payload']['count']))+']'+r['payload']['suffix']
            html=''.join(data['lines'][i] for i in r['before'])+payload+''.join(data['lines'][i] for i in r['after'])
            assert sha(payload.encode())==expected[key]['payload_sha256']
            assert sha(html.encode())==expected[key]['html_sha256'] and html.encode()==(source/f'flight-{key}.html').read_bytes()
            assert r['evidence']==json.loads((source/f'flight-{key}-evidence.json').read_text()) and r['proof']==json.loads((source/f'flight-{key}-proof.json').read_text())
            assert index[key]['pack']==chunk['path'];checked.append(key)
    assert len(checked)==len(set(checked))==declared['rich_details']==len(index)
    for name in {v['previews'] for v in index.values()}:
        for key,svg in json.loads(gzip.decompress((root/name).read_bytes())).items():assert svg==(source/f'flight-{key}.svg').read_text()
    result={'passed':True,'rich_details':len(checked),'chunks':len(declared['chunks']),'html_and_numeric_literals_exact':True,'proof_and_evidence_exact':True,'preview_svg_exact':True,'packing_receipt_sha256':sha((root/'packing-receipt.json').read_bytes()),'native_transitions':0}
    with a.receipt.open('x') as stream:json.dump(result,stream,indent=2);stream.write('\n')
    print(json.dumps(result),flush=True)


if __name__=='__main__':main()
