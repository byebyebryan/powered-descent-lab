"""Package maintained saved-batch renderings and verify their existing scalars."""
import argparse
from collections import Counter
import hashlib
from html import escape
from html.parser import HTMLParser
import json
from pathlib import Path
import re
from urllib.parse import urljoin, urlparse


def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()


class Rows(HTMLParser):
    def __init__(self,text):
        super().__init__();self.stack=[];self.rows=[];self.feed(text)

    def handle_starttag(self,tag,attrs):
        attrs=dict(attrs)
        if tag=='tr':self.stack.append({'attrs':attrs,'cells':[],'cell':None})
        elif tag in ('td','th') and self.stack:self.stack[-1]['cell']=[]

    def handle_data(self,data):
        if self.stack and self.stack[-1]['cell'] is not None:self.stack[-1]['cell'].append(data)

    def handle_endtag(self,tag):
        if not self.stack:return
        row=self.stack[-1]
        if tag in ('td','th') and row['cell'] is not None:
            row['cells'].append(' '.join(''.join(row['cell']).split()));row['cell']=None
        elif tag=='tr':self.rows.append(self.stack.pop())

    def scientific(self):
        rows=[]
        for row in self.rows:
            attrs=row['attrs'];classes=attrs.get('class','').split()
            if not any(c in classes for c in ['summary-row','seed-row']):continue
            rows.append((attrs.get('data-kind'),attrs.get('data-group'),attrs.get('data-parent'),row['cells'][:6]))
        return rows


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('rendered',type=Path);p.add_argument('target',type=Path)
    a=p.parse_args();repo=Path(__file__).resolve().parents[2];a.target.mkdir();sources=[]
    for short,pack in [('terminal','terminal_bot_lab_suite'),('transfer','transfer_route_angle_radius_suite')]:
        source=repo/'outputs/eval'/pack;old=(source/'report.html').read_text()
        fresh=(a.rendered/(short+'.html')).read_text();summary=json.loads((source/'summary.json').read_text())
        old_rows=Rows(old).scientific();new_rows=Rows(fresh).scientific()
        assert old_rows==new_rows,(pack,'saved scalar/selector rows differ')
        records=summary['records'];seeds=Counter(record['resolved']['resolved_seed'] for record in records)
        archive=f'http://starship:8000/eval/{pack}/'
        # Old retention removed raw samples and rich pages. A fresh renderer
        # therefore has no aggregate previews to rebuild. Preserve the exact
        # retained HTML/SVG edition after checking current scalar semantics.
        retained_previews=old.count('class="run-preview lane-preview"')
        assert retained_previews>0
        text=re.sub(r'\s*<base href="[^"]*"\s*/?>','',old,count=1)
        missing=[]
        def retained_link(match):
            href=match[1]
            if re.fullmatch(r'runs/[^/]+/report\.html',href) and not (source/href).is_file():
                missing.append(href)
                return 'data-unavailable-href="'+escape(urljoin(archive,href),quote=True)+'" aria-disabled="true" title="Individual flight detail is unavailable in this retained archive"'
            return match[0]
        text=re.sub(r'href="([^"]+)"',retained_link,text)
        text=re.sub(r'href="([^"]+)"',lambda m:m[0] if m[1].startswith('#') or '${' in m[1] else 'href="'+escape(urljoin(archive,m[1]),quote=True)+'"',text)
        note=' Aggregate trajectories and per-seed summaries are retained; individual flight detail and raw samples are unavailable in this archive.' if missing else ''
        banner='<nav aria-label="Seeded case editions" style="padding:8px 20px;font:13px/1.5 system-ui;background:#fffdf8">Retained seeded cases · <a href="terminal.html">Terminal</a> · <a href="transfer.html">Transfer</a> · <a href="'+archive+'report.html">Original retained report (8000)</a>'+note+'</nav>'
        text=text.replace('<body>','<body>'+banner,1)
        with (a.target/(short+'.html')).open('x') as stream:stream.write(text)
        assert text.count('class="run-preview lane-preview"')==retained_previews
        sources.append({'pack':pack,'records':len(records),'seed_counts':dict(seeds),'scalar_selector_rows':len(new_rows),'retained_previews':retained_previews,'fresh_preview_rebuild_available':fresh.count('class="run-preview lane-preview"')>0,'missing_detail_links':missing,'summary_sha256':sha(source/'summary.json'),'original_report_sha256':sha(source/'report.html'),'native_transitions':0})
    result={'passed':True,'sources':sources,'rendered':str(a.rendered.resolve()),'assets':{p.name:{'sha256':sha(p),'bytes':p.stat().st_size} for p in sorted(a.target.iterdir())}}
    with (a.target.parent/'seeded-export.json').open('x') as stream:json.dump(result,stream,indent=2);stream.write('\n')
    brief=[{**{k:v for k,v in source.items() if k!='missing_detail_links'},'unavailable_details':len(source['missing_detail_links'])} for source in sources]
    print(json.dumps({'passed':True,'sources':brief,'bytes':sum(v['bytes'] for v in result['assets'].values())}))


if __name__=='__main__':main()
