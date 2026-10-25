"""Create a source-bound display revision, reusing immutable published data."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import shutil


def digest(data):return hashlib.sha256(data).hexdigest()


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('target',type=Path);p.add_argument('previous_module',type=Path);p.add_argument('--name',default='responsive')
    a=p.parse_args();repo=Path(__file__).resolve().parents[2];target=a.target.resolve()
    before=a.previous_module.read_text();module=repo/'pd-report/src/flight_inspection.js';after=module.read_text()
    assert before!=after
    export=json.loads((target/'export-receipt.json').read_text());packed=Path(export['packing'])
    checked=0
    # The complete rich template inventory must contain the exact old display
    # module once, outside every scientific payload. No template is rewritten.
    for path in sorted((packed/'data').glob('flights-*.json.gz')):
        pack=json.loads(gzip.decompress(path.read_bytes()))
        for record in pack['cases'].values():
            template=''.join(pack['lines'][i] for i in record['before'])+'__PAYLOAD__'+''.join(pack['lines'][i] for i in record['after'])
            assert template.count(before)==1,(path,checked)
            assert before not in record['payload']['prefix']+record['payload']['suffix']
            checked+=1
    pair=gzip.decompress((packed/'data/compare.html.gz').read_bytes()).decode();assert pair.count(before)==1
    assert a.name and all(c.isalnum() or c=='-' for c in a.name)
    ui=target/('ui-'+a.name);ui.mkdir()
    for path in (target/'ui-publish').iterdir():
        if path.name=='asset-manifest.json':continue
        if path.is_dir():shutil.copytree(path,ui/path.name)
        else:shutil.copyfile(path,ui/path.name)
    shutil.copyfile(repo/'pd-report/src/portable_assets.js',ui/'assets/portable_assets.js')
    patch={'version':1,'scope':'inspection display module only; canonical payloads and data bundles unchanged','before_sha256':digest(before.encode()),'after_sha256':digest(after.encode()),'before':before,'after':after}
    with (ui/'assets/template-patch.json').open('x') as stream:json.dump(patch,stream);stream.write('\n')
    manifest=json.loads((target/'ui-publish/asset-manifest.json').read_text())
    manifest['presentation_patch']='assets/template-patch.json'
    with (ui/'asset-manifest.json').open('x') as stream:json.dump(manifest,stream,indent=2);stream.write('\n')
    receipt={'passed':True,'previous_ui':str(target/'ui-publish'),'ui':str(ui),'data_aliases_unchanged':True,'canonical_html_unchanged':True,'checked_rich_templates':checked,'checked_pair_templates':1,'old_module':str(a.previous_module.resolve()),'new_module':str(module),'old_module_sha256':patch['before_sha256'],'new_module_sha256':patch['after_sha256'],'loader_sha256':digest((ui/'assets/portable_assets.js').read_bytes()),'assets':{str(path.relative_to(ui)):digest(path.read_bytes()) for path in sorted(ui.rglob('*')) if path.is_file()}}
    with (target/(a.name+'-ui-receipt.json')).open('x') as stream:json.dump(receipt,stream,indent=2);stream.write('\n')
    print(json.dumps({k:v for k,v in receipt.items() if k!='assets'}),flush=True)


if __name__=='__main__':main()
