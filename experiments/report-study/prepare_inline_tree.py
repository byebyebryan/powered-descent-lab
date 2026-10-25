"""Create a bounded source-bound tree UI edition over immutable saved data."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import shutil
import subprocess


def digest(data):return hashlib.sha256(data).hexdigest()


def read(path):return json.loads(path.read_text())


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('previous',type=Path);p.add_argument('target',type=Path);p.add_argument('spec',type=Path)
    a=p.parse_args();previous=a.previous.resolve();target=a.target.resolve();spec=read(a.spec)
    assert spec['schema_version']==1 and spec['patches']
    repo=Path(__file__).resolve().parents[2];head=subprocess.check_output(['rtk','proxy','git','rev-parse','HEAD'],cwd=repo,text=True).strip()
    assert spec['source_head']==head,'Freeze and bind the selected source commit first'
    export=read(previous/'export-receipt.json');packed=Path(export['packing'])
    old_ui=previous/'ui-final';manifest=read(old_ui/'asset-manifest.json')
    patches=[];sources={}
    for item in spec['patches']:
        if 'before_text' in item:
            assert 'before_path' not in item and 'after_paths' not in item
            before=item['before_text'];after=item['after_text'];paths=[]
            assert isinstance(before,str) and isinstance(after,str)
        else:
            before_path=Path(item['before_path']).resolve();before=before_path.read_text()
            paths=[repo/name for name in item['after_paths']]
            after='\n'.join(path.read_text() for path in paths)
            paths=[before_path,*paths]
        assert before and after and before!=after and item['views']
        for path in paths:sources[str(path)]={'sha256':digest(path.read_bytes()),'bytes':path.stat().st_size}
        patches.append({'id':item['id'],'views':item['views'],'before':before,'after':after,'before_sha256':digest(before.encode()),'after_sha256':digest(after.encode())})
    assert len({p['id'] for p in patches})==len(patches)

    def apply(parts,view):
        for patch in patches:
            expected=int(view in patch['views']);count=sum(part.count(patch['before']) for part in parts)
            assert count==expected,(patch['id'],view,count,expected)
            if count:parts=[part.replace(patch['before'],patch['after']) for part in parts]
        return parts

    # Preflight all exact source templates; never unpack/rewrite flight values.
    checked=0
    for path in sorted((packed/'data').glob('flights-*.json.gz')):
        pack=json.loads(gzip.decompress(path.read_bytes()))
        for record in pack['cases'].values():
            parts=[''.join(pack['lines'][i] for i in record['before']),''.join(pack['lines'][i] for i in record['after'])]
            apply(parts,'rich');checked+=1
    for name,view in [('report.html','study'),('compare.html','pair')]:
        apply([gzip.decompress((packed/'data'/(name+'.gz')).read_bytes()).decode()],view)

    target.mkdir();ui=target/'ui';shutil.copytree(old_ui,ui)
    shutil.copyfile(repo/'pd-report/src/portable_assets.js',ui/'assets/portable_assets.js')
    with (ui/'assets/inline-tree-patch.json').open('x') as stream:json.dump({'version':2,'patches':patches},stream);stream.write('\n')
    manifest['presentation_patch']='assets/inline-tree-patch.json'
    # This is a new UI copy; previous manifests and saved data remain untouched.
    (ui/'asset-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    assert manifest['assets']==read(old_ui/'asset-manifest.json')['assets']
    provenance={'source_head':head,'kind':'inline tree display revision','previous_ui_report':read(previous/'publication-ui-final.json')['url'],'native_transitions':0,'source_replays':0,'training_updates':0,'exact_template_preflight_rich_details':checked,'source_files':sources}
    with (ui/'inline-tree-provenance.json').open('x') as stream:json.dump(provenance,stream,indent=2);stream.write('\n')
    assets={str(path.relative_to(ui)):{'sha256':digest(path.read_bytes()),'bytes':path.stat().st_size} for path in sorted(ui.rglob('*')) if path.is_file()}
    receipt={'passed':True,'source_head':head,'previous':str(previous),'previous_ui':str(old_ui),'ui':str(ui),'packing':str(packed),'canonical_source':export['source'],'data_aliases_unchanged':True,'canonical_html_untouched':True,'preflight_rich_details':checked,'new_bytes':sum(p['bytes'] for p in assets.values()),'assets':assets,'sources':sources,'patch_spec':str(a.spec.resolve()),'patch_spec_sha256':digest(a.spec.read_bytes()),'native_transitions':0}
    assert receipt['new_bytes']<32*1024**2
    with (target/'export-receipt.json').open('x') as stream:json.dump(receipt,stream,indent=2);stream.write('\n')
    print(json.dumps({k:v for k,v in receipt.items() if k not in ['assets','sources']}),flush=True)


if __name__=='__main__':main()
