"""Lossless column dictionary for native JSON sample literals.

Preserve every numeric token, including negative zero and exponent spelling.
Only presentation bytes are compressed; no trajectories are simplified.
"""
import gzip,hashlib,json,re

DECODER=json.JSONDecoder()
NUMBER=re.compile(r'-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?\Z')

def flip(token):
    if float(token)==0:return token
    return token[1:] if token.startswith('-') else '-'+token

def split_payload(payload):
    start=payload.index('"samples":')+len('"samples":')
    values,end=DECODER.raw_decode(payload,start)
    text=payload[start:end];i=1;keys=None;columns=[]
    for row in values:
        assert text[i]=='{';i+=1;local_keys=[];tokens=[]
        for ordinal in range(len(row)):
            key,stop=DECODER.raw_decode(text,i);literal=text[i:stop];i=stop
            assert text[i]==':';i+=1;value,stop=DECODER.raw_decode(text,i);tokens.append(text[i:stop]);i=stop;local_keys.append(literal)
            if ordinal+1<len(row):assert text[i]==',';i+=1
        assert text[i]=='}';i+=1
        if keys is None:keys=local_keys;columns=[[] for _ in keys]
        assert keys==local_keys
        for column,token in zip(columns,tokens):
            assert '\n' not in token;column.append(token)
        if i<len(text)-1:assert text[i]==',';i+=1
    assert i==len(text)-1 and text[i]==']'
    return payload[:start],payload[end:],keys or [],columns,len(values)

def pack(payload,pool):
    prefix,suffix,keys,columns,count=split_payload(payload);refs=[]
    for values in columns:
        raw='\n'.join(values);flipped=False
        if values and all(NUMBER.fullmatch(v) for v in values):
            other='\n'.join(map(flip,values))
            if other<raw:raw=other;flipped=True
        ident=hashlib.sha256(raw.encode()).hexdigest();pool.setdefault(ident,raw);refs.append([ident,flipped])
    result={'prefix':prefix,'suffix':suffix,'keys':keys,'columns':refs,'count':count}
    assert unpack(result,pool)==payload
    return result

def unpack(record,pool):
    columns=[]
    for ident,flipped in record['columns']:
        values=pool[ident].split('\n')
        columns.append([flip(v) for v in values] if flipped else values)
    rows=['{'+','.join(key+':'+columns[j][i] for j,key in enumerate(record['keys']))+'}' for i in range(record['count'])]
    return record['prefix']+'['+','.join(rows)+']'+record['suffix']

def compressed(value):return gzip.compress(json.dumps(value,separators=(',',':'),ensure_ascii=False,allow_nan=False).encode(),compresslevel=6,mtime=0)

def html_tokens(text):
    # Tags and line breaks are dictionary boundaries only; joining the chunks
    # must reproduce the exact maintained document, including inline scripts.
    chunks=[part for part in re.split(r'(<[^>]*>|\n)',text) if part]
    assert ''.join(chunks)==text
    return chunks

def write_batch_page(export,html):
    body=gzip.compress(html.encode(),compresslevel=6,mtime=0)
    assert gzip.decompress(body).decode()==html
    with (export/'report-body.html.gz').open('xb') as f:f.write(body)
    shell='''<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Complete terminal-entry experiment batch</title><script src="assets/fflate-0.8.2.min.js"></script></head><body style="font:16px system-ui;padding:24px"><p id="loading">Loading the complete mission batch…</p><script>(async()=>{try{const r=await fetch('report-body.html.gz');if(!r.ok)throw Error('HTTP '+r.status);const html=fflate.strFromU8(fflate.gunzipSync(new Uint8Array(await r.arrayBuffer())));document.open();document.write(html);document.close()}catch(error){document.getElementById('loading').textContent='Batch could not load: '+error.message}})();</script></body></html>'''
    with (export/'report.html').open('x') as f:f.write(shell)
