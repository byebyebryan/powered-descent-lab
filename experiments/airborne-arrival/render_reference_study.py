"""Established rich mission tree with frozen-reference qualification analysis."""
import argparse
import html
import json
from pathlib import Path

import pd_coast_native as native
from arrival import read, write_new

FIRST,FUEL,EXTENDED="first_feasible_v1","minimum_fuel_grid_v1","minimum_fuel_extended_grid_v1"
NAMES={FIRST:"Legacy first feasible",FUEL:"Fuel-ranked 63 pairs",EXTENDED:"Frozen reference · 132 pairs"}


def context(info,mode,phase):
    search=info["landing_search"]
    pair="No feasible timing selected" if search["chosen_coast_ticks"] is None else (
        f'Coast/attitude preparation {search["chosen_coast_ticks"]/120:.3f} s; accepted Poly duration {search["chosen_duration_ticks"]/120:.3f} s')
    arrival=info["arrival"]
    state="Direct landing: no intermediate instruction." if arrival is None else (
        f'Original arrival instruction {arrival["status"]} at {arrival["resolution"]["sim_time_s"]:.3f} s. '
        'Continuation starts at this actual native state, preserving fuel, attitude, angular rate and the original mission clock.')
    return f'''<section><h2>Frozen timing reference</h2><p>{html.escape(phase)} · {html.escape(NAMES[mode])}.</p>
<p>{html.escape(state)} {html.escape(pair)}.</p>
<p>The vehicle, plant, physical contact criteria and Poly burn arithmetic are fixed. Minimum-fuel profiles enumerate their entire named finite bank, using native attitude preparation and contact forecasts. This is a finite-bank reference, not a global optimum. The legacy default is unchanged.</p>
<p>Incoming contact is measured before native final-velocity reset. Timing is measured in a separate repeated exposed-state benchmark; this captured flight has no clock instrumentation. Preview axes fit the full flight and pad with equal metre scales shared across choices for this physical state.</p></section>'''


def overview(a,audit,rows):
    chosen=a["chosen_reference"];primary=a["populations"]["primary"];boundary=a["populations"]["boundary"]
    gain=primary["paired"][FIRST+"__"+chosen];extension=primary["paired"][FUEL+"__"+chosen]
    selected=primary["profiles"][chosen]
    coasts=[float(k) for k in selected["selected_coast_seconds"]]
    duration14=selected["selected_duration_seconds"].get("14.0",0)
    headline=(f'{selected["verified_landings"]}/300 frozen-reference landings verify. ' +
        (f'The one-sided 95% exact lower bound is {100*selected["lower_bound_95"]:.4f}%, passing the declared 99% criterion.' if a["primary_99_percent_gate_passed"] else 'The declared 99% reliability criterion fails.'))
    table=""
    for mode,p in primary["profiles"].items():
        b=boundary["profiles"][mode];fuel=p["fuel_successes_kg"]
        table+=f'<tr><td>{NAMES[mode]}</td><td>{p["verified_landings"]}/300</td><td>{b["verified_landings"]}/60</td><td>{fuel["mean"]:.2f}</td><td>{fuel["median"]:.2f}</td><td>{fuel["p95"]:.2f}</td></tr>'
    cost=""
    for mode,c in a["costs"].items():
        s,o=c["selection_ms"],c["ordinary_frame_us"]
        cost+=f'<tr><td>{NAMES[mode]}</td><td>{s["median"]:.2f} / {s["p95"]:.2f} / {s["max"]:.2f}</td><td>{o["median"]:.2f} / {o["p95"]:.2f} / {o["max"]:.2f}</td><td>{c["episode_process_cpu_ms"]["median"]:.2f}</td></tr>'
    contact=""
    for mode,p in primary["profiles"].items():
        c=p["contact"]
        contact+=f'<tr><td>{NAMES[mode]}</td><td>{c["speed_mps"]["max"]:.6f}</td><td>{c["lateral_mps"]["max"]:.6f}</td><td>{c["attitude_rad"]["max"]:.6f}</td><td>{c["angular_rate_radps"]["max"]:.6f}</td><td>{c["pad_offset_m"]["max"]:.6f}</td></tr>'
    conditional=' Fuel summaries below are conditional on successful pairs.' if gain["conditional"] else ''
    payload=json.dumps(rows,separators=(',',':'),allow_nan=False).replace('<','\\u003c')
    return f'''<section><h2>Independent timing-reference qualification</h2><p><strong>{headline}</strong></p>
<p>The independent primary population contains 300 input-only draws from a uniform four-family × three-sink-band mixture, with continuous jitter and no cell quotas. The 60 predeclared endpoint cases and 21 retained arrival regressions have separate denominators. Every scheduled comparison has full captured proof, including any miss; no outcome admission, replacements, learner or RL updates.</p>
<div style="overflow:auto"><table><thead><tr><th>Timing profile</th><th>Primary verified</th><th>Boundary verified</th><th>Mean fuel kg</th><th>Median kg</th><th>P95 kg</th></tr></thead><tbody>{table}</tbody></table></div>
<p>Paired reference saving versus legacy: {gain["mean_saving_kg"]:.2f} kg ({100*gain["saving_fraction"]:.2f}%; paired original-level bootstrap 95% interval {100*gain["saving_fraction_ci95"][0]:.2f}–{100*gain["saving_fraction_ci95"][1]:.2f}%). The larger bank saves an additional {extension["mean_saving_kg"]:.2f} kg ({100*extension["saving_fraction"]:.2f}%; interval {100*extension["saving_fraction_ci95"][0]:.2f}–{100*extension["saving_fraction_ci95"][1]:.2f}%) versus 63 pairs.{conditional} Successful pairs: {gain["successful_pairs"]}/300 and {extension["successful_pairs"]}/300 respectively. Crashes never earn fuel credit.</p>
<p><img src="qualification.svg" alt="Paired fuel consumption, savings distribution and selection latency for the three frozen profiles" style="max-width:100%;height:auto"></p></section>
<section><h2>What was frozen before these results</h2><p>The 132-pair bank adds coast seconds 5, 7, 8, 9, 10 and burn seconds 14, 18 to the 63-pair Cartesian grid. All other coast/duration pairs, native forecast rules and fuel ranking are preserved. The expanded candidate landed all 60 exposed development originals and saved {100*a["development"]["mean_saving_fraction"]:.2f}% mean fuel against 63 pairs, passing the predeclared 1% selection rule. Both old profiles reproduce all 180 retained programs exactly, for 360 preservation executions. The reference and generator hashes were frozen before primary/boundary generation.</p>
<p>The fixed envelope is height 725–875 m, horizontal position zero or 310–390 m from the pad, lateral speed zero/22–28/37–43 m/s, sink in 13–17/28–32/43–47 m/s bands and modest family-conditioned attitude. Initial angular rate is zero; vehicle, initial fuel (6300 kg), Earth gravity and flat terrain are fixed. This is a population-specific statement, not family-wise 99% reliability, arbitrary landing control, repeated retargeting or uncertain-dynamics robustness.</p>
<p>The selected primary coast ranges from {min(coasts):g} to {max(coasts):g} s, below the expanded 10 s scan boundary. The newly available 14 s burn duration is selected on {duration14}/300 originals. Timing and burn duration matter jointly; this result does not establish that later ignition is always better, or that the finite grid is continuously optimal.</p>
<p>Boundary engineering gate: {str(a["boundary_engineering_gate_passed"]).lower()}; retained arrival regression gate: {str(a["regression_gate_passed"]).lower()}. All 21 retained native prefixes are checked through actual first arrival resolution. The 30 s arrival deadline, goal tolerances and 0.72 rad arrival tilt bound remain fixed. There is no default promotion.</p></section>
<section><h2>Planning cost and local control cost</h2><p>Sequential isolated measurements use 12 exposed originals, ten repetitions per profile: first invocation, two warmups, seven measured repeats, with deterministic rotated order. Each profile has 84 measured selections. First invocation is not an OS-cold-start claim. Raw times and host/load context are retained.</p>
<div style="overflow:auto"><table><thead><tr><th>Profile</th><th>Selection ms median / P95 / max</th><th>Ordinary frame µs median / P95 / max</th><th>Episode CPU ms median</th></tr></thead><tbody>{cost}</tbody></table></div>
<p>Selection times cover the timing search alone; ordinary frame times cover controller-frame computation and exclude native plant stepping, actuator allocation, persistence and Python overhead. These are descriptive wall measurements on one host, with operating-system outliers. Selection belongs to planning: a 60 Hz command interface does not imply this search fits a 16.7 ms deadline. Whole-episode CPU and forecast work remain separate in the analysis data. Measurements are sidecars and leave flight decisions/fingerprints unchanged.</p></section>
<section><h2>Incoming contact and proof</h2><p>Maximum absolute primary contact quantities, measured before the final-state velocity reset:</p><div style="overflow:auto"><table><thead><tr><th>Profile</th><th>Speed m/s</th><th>Lateral m/s</th><th>Attitude rad</th><th>Angular rate rad/s</th><th>Pad offset m</th></tr></thead><tbody>{contact}</tbody></table></div>
<p>Native stable-contact limits remain normal speed 3 m/s, tangential speed 2 m/s, attitude error 0.15 rad and angular rate 0.35 rad/s, together with pad/hull/mission predicates. The full incoming state and physical classification are available in every rich flight and raw capture.</p>
<p>{audit["captures"]:,} new captures have physical, mission, integrity and complete neutral replay evidence plus two deterministic reconstructions each. The read-only audit verifies exact schedules, source/library/input hashes, finite-bank ranking, selected forecast versus actual contact, first arrival predicates, retained prefixes and all {audit["total_steps"]:,} charged steps. One development reservation stop is retained: 58 captures completed before an interrupted Summary consumed two native ticks and 90,894 forecast steps. A separately allocated phase completed the exact remaining schedule; no inputs, selector or flight logic changed. The attempt is charged and makes no landing claim.</p>
<p>The mission tree contains all fresh comparisons, endpoint cases, handoff regressions, exposed three-profile comparisons and four labelled pilot repeats. It preserves the established rich flight template, focused equal-metre previews, bounded tree browsing and raw evidence links. The next decision can use this reference to assess genuinely new planner choices; this pass provides no evidence that RL is needed for the timing menu.</p>
<p><a href="analysis.json">Analysis data</a> · <a href="evidence-audit.json">Evidence audit</a> · <a href="reference-freeze.json">Reference freeze</a> · <a href="http://starship:8000/arrival-timing-selection-report-20261024T171406Z/report.html">Previous timing and intermediate-goal report</a></p></section>
<section><h2>Compare profiles on one original</h2><label>Original state <select id="reference-state" style="display:block;width:680px;max-width:100%"></select></label><div style="overflow:auto"><table><thead><tr><th>Timing profile</th><th>Verified landing</th><th>Fuel kg</th><th>Coast s</th><th>Burn s</th><th>Flight</th></tr></thead><tbody id="reference-results"></tbody></table></div></section>
<script>(()=>{{const rows={payload},select=document.getElementById('reference-state'),body=document.getElementById('reference-results');for(const id of [...new Set(rows.map(r=>r.state_id))]){{const o=document.createElement('option');o.value=id;o.textContent=id;select.append(o)}}function render(){{body.replaceChildren();for(const r of rows.filter(r=>r.state_id===select.value)){{const tr=document.createElement('tr');for(const value of [r.name,String(r.verified),r.result.fuel_used_kg.toFixed(2),r.result.coast_ticks===null?'none':(r.result.coast_ticks/120).toFixed(2),r.result.duration_ticks===null?'none':(r.result.duration_ticks/120).toFixed(2)]){{const td=document.createElement('td');td.textContent=value;tr.append(td)}}const td=document.createElement('td'),link=document.createElement('a');link.href=r.href;link.textContent='Rich flight';td.append(link);tr.append(td);body.append(tr)}}}}select.addEventListener('change',render);render()}})();</script>'''


def render(root,output):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    import numpy as np
    import shutil
    a,audit=read(root/'analysis.json'),read(root/'evidence-audit.json')
    specs,rows=[],[]
    phase_names={"primary":"Primary independent mixture","boundary":"Boundary endpoint panel","regressions":"Retained arrival regressions","development":"Exposed development","development-completion":"Exposed development","pilot":"Cost pilot repeats"}
    for phase in ["primary","boundary","regressions","development","development-completion","pilot"]:
        result=read(root/phase/('fault.json' if phase=='development' else 'result.json'))
        lookup={(r['case'],r['mode']):r for r in result['rows']}
        for capture in result['captures']:
            row=lookup[(capture['case'],capture['mode'])]; folder=Path(capture['capture'])
            specs.append({'capture':str(folder),'population':phase_names[phase]+' · '+row['population'].split('_',1)[-1].replace('_',' '),
                'label':NAMES[row['mode']],'context_html':context(read(folder/'decisions.json'),row['mode'],phase_names[phase])})
            if phase in ['primary','boundary']:
                rows.append({**row,'name':NAMES[row['mode']],'verified':capture['verified_target_landing'],'href':f'flight-{len(specs)-1}.html'})
    # Retained old direct captures complete each exposed three-profile comparison.
    old=Path('outputs/arrival-timing-selection/loop-20261024T171406Z').resolve()
    old_composition=Path(read(old/'comparison-allocation.json')['retained_root'])
    fuel={c['case']:c for p in ['comparison','proof-completion'] for c in read(old/p/'result.json')['captures']}
    for row in read(old/'comparison/result.json')['rows']:
        if row['choice']!='direct':continue
        folder=old_composition/'confirmation/captures'/(row['case']+'-native_pair_v2') if row['mode']==FIRST else Path(fuel[row['case']]['capture'])
        specs.append({'capture':str(folder),'population':'Exposed development · '+row['population'].removeprefix('confirmation_').replace('_',' '),
            'label':NAMES[row['mode']]+' · retained exact execution','context_html':context(read(folder/'decisions.json'),row['mode'],'Exposed development; retained capture exactly reproduced')})
    spec={'title':'Timing reference qualification','subtitle':'300 independent originals · 60 endpoint cases · 21 retained handoffs · complete rich flight evidence',
        'summary':f'{a["populations"]["primary"]["profiles"][a["chosen_reference"]]["verified_landings"]}/300 frozen-reference landings verify; separate efficiency, boundary and timing evidence.',
        'grouped':False,'focus_flat':True,'max_bytes':4*1024**3,'overview_html':overview(a,audit,rows),'rows':specs}
    write_new(root/'report-spec.json',spec)
    native.render_report(json.dumps(spec,allow_nan=False),str(output))
    for name in ['analysis.json','evidence-audit.json','reference-freeze.json']:shutil.copy2(root/name,output/name)
    fig,axes=plt.subplots(3,1,figsize=(10,11),layout='constrained')
    colors=['#8793a3','#4171a0','#128679'];modes=[FIRST,FUEL,EXTENDED]
    means=[a['populations']['primary']['profiles'][m]['fuel_successes_kg']['mean'] for m in modes]
    bars=axes[0].bar(range(3),means,color=colors);axes[0].bar_label(bars,fmt='%.1f',padding=3)
    axes[0].set(xticks=range(3),xticklabels=['Legacy','63 pairs','132 pairs'],ylabel='Mean mission fuel (kg)',ylim=(0,max(means)*1.18))
    primary=read(root/'primary/result.json')['rows'];by={}
    for row in primary:by.setdefault(row['state_id'],{})[row['mode']]=row['result']
    for mode,color in [(FUEL,colors[1]),(EXTENDED,colors[2])]:
        savings=sorted(v[FIRST]['fuel_used_kg']-v[mode]['fuel_used_kg'] for v in by.values() if v[FIRST]['success'] and v[mode]['success'])
        axes[1].plot(np.linspace(0,100,len(savings)),savings,label=NAMES[mode],color=color)
    axes[1].set(xlabel='Successful-pair saving percentile',ylabel='Fuel saved vs legacy (kg)');axes[1].legend()
    for i,mode in enumerate(modes):
        c=a['costs'][mode]['selection_ms'];axes[2].plot([i,i],[c['p05'],c['p95']],color=colors[i],linewidth=4);axes[2].scatter([i],[c['median']],color=colors[i],s=60)
    axes[2].set(xticks=range(3),xticklabels=['Legacy','63 pairs','132 pairs'],ylabel='Selection wall time (ms)',title='Median and P05–P95; 84 measured selections per profile')
    for ax in axes:ax.grid(axis='y',alpha=.2)
    fig.savefig(output/'qualification.svg');plt.close(fig)
    write_new(root/'report-inventory.json',{'rich_occurrences':len(specs),'comparison_rows':len(rows),'physical_originals':441,'retained_captures_reused':120})


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('output',type=Path);args=p.parse_args()
    render(args.root.resolve(),args.output.resolve())
