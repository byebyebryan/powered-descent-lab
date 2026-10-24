"""Display completed composition evidence through the established rich tree."""
import argparse
import html
import math
from pathlib import Path
from composition_study import encoded, native, read, sha, write_new

PHASES=["pilot","witnesses","diagnostics","development","confirmation"]


def context(info,mode,phase):
    a=info["arrival"];s=info["landing_search"]
    text=f'<section><h2>Arrival and landing execution</h2><p>{html.escape(phase)} · {html.escape(mode)}. '
    if a:
        o=a["resolution"]
        text+=f'Arrival <strong>{html.escape(a["status"])}</strong> at {o["sim_time_s"]:.3f} s; '
        text+=f'actual handoff fuel {o["fuel_kg"]:.2f} kg, attitude {math.degrees(o["attitude_rad"]):.2f}°, angular rate {math.degrees(o["angular_rate_radps"]):.2f}°/s. '
        text+='The unchanged Poly selector starts at this actual resolution; the original plant and clocks continue.</p>'
        if a.get("actuation"):
            text+=f'<p>Allocation updates: {html.escape(str(a["actuation"]["regime_updates"]))}. Local horizon remaining {a["deadline_remaining_s"]:.3f} s; operation time remaining at final arrival command {a["actuation"]["operation_remaining_s"]:.3f} s.</p>'
    else:text+='Direct Poly comparison; no intermediate arrival.</p>'
    if s["chosen_duration_ticks"] is not None:
        text+=f'<p>Selected coast/preparation {s["chosen_coast_ticks"]/120:.3f} s, followed by a {s["chosen_duration_ticks"]/120:.3f} s Poly reference. First feasible in the fixed coast/duration grid; no fuel optimization.</p>'
    else:text+=f'<p>Finite continuation search found no plan: {s["no_plan"]}. This is not a physical impossibility certificate.</p>'
    if phase=="diagnostics":text+='<p>This exposed retained failure is outside the newly confirmed menu/envelope. Its complete decision info and execution fingerprint remain unchanged. It is not counted as a confirmation success.</p>'
    text+='<p>Local instruction fulfillment and verified landing are separate outcomes. Per-update CPU timing is uninstrumented; zero values in shared performance cards are not measured zero-cost control.</p></section>'
    return text


def overview(a,audit,gate,rows):
    q,h=a["confirmation"],a["headroom"]
    table=''.join(f'<tr><td>{html.escape(f.removeprefix("confirmation_").replace("_"," "))}</td>'
        +''.join(f'<td>{v[ch]["native_success"]}/{v[ch]["programs"]} · {v[ch]["mean_fuel_kg"]:.2f} kg</td>' for ch in ['direct','support_short','brake_long'])+'</tr>' for f,v in q["family_summary"].items())
    counts=h["best_observed_choice_counts"]
    payload=encoded(rows).replace('<','\\u003c')
    return f'''<section><h2>Existing corrected control passes the bounded composition gate</h2>
<p><strong>{q["arrival_and_verified_landing"]}/120 fresh arrival programs fulfill their instructions and land; all {q["direct_verified_landings"]}/60 paired direct controls land.</strong> Every confirmation execution carries complete physical/mission/integrity/replay proof, controller reconstruction, input-goal reconstruction and Summary equality. The 120 arrival programs share 60 independent original states.</p>
<p>The original 21 corrected arrivals and all 21 independent native witnesses also land from their first actual fulfillment states. All 44 pilot/development-prefix captures exactly preserve old native samples through handoff and commands before it. No guidance repair, default promotion or RL training was needed.</p>
<p>One optional arrival is followed by the existing first-feasible Poly Landing selector. The controller owns commands. Flat terrain, the current vehicle/Earth/120–60 Hz plant, 0.72 rad arrival tilt limit and original tolerances remain fixed. The first actual handoff preserves position, velocity, fuel, attitude/angular rate and clocks; it is not the six-second witness endpoint.</p></section>
<section><h2>Qualified task and its limits</h2>
<p>The frozen input population spans vertical, lateral, braking and modest attitude-preparation states: height 725–875 m; horizontal position 0 or 310–390 m from the pad; lateral speed 0, 22–28 or 37–43 m/s; sink 13–47 m/s; initial attitude within about ±0.29 rad. Fuel and vehicle are fixed. The recipes define goals through independent native endpoints; goal tolerances remain 30 m, ±15% speed, 0.25 rad heading and bounded cross-speed.</p>
<p><code>support_short</code> constructs a goal from a 5.5–6.5 s gravity-supported native endpoint. <code>brake_long</code> uses a 7–8 s endpoint with 35% nominal lateral-velocity reduction. Attitude-preparation recipes coast/point for 0.5 s first. These times construct the goals; they do not prescribe arrival time. Arrival has an absolute 30 s deadline within the original 90 s mission. The planner receives position/velocity goals; witness commands are evaluation evidence, not a learned action.</p>
<p>Twelve separate development states selected the fixed baseline (direct landing). The controller, generator and goal rules were frozen before all 60 confirmation states; there were no exclusions, replacements or confirmation-driven retuning. This is a bounded engineering confirmation, not a 99% population-reliability result, arbitrary state control or repeatedly retargeted guidance.</p>
<div style="overflow:auto"><table><thead><tr><th>Confirmation family</th><th>Direct verified / mean fuel</th><th>Support short verified / mean fuel</th><th>Brake long verified / mean fuel</th></tr></thead><tbody>{table}</tbody></table></div>
<p>The 14 exposed difficult programs reproduce their old complete decision info and fingerprints: two fulfill arrival but cannot complete landing; three land without fulfilling arrival. The high-energy shallow-entry gap and projection/minimum-thrust/recovery concerns remain. These diagnostic failures are retained separately from the newly confirmed envelope.</p></section>
<section><h2>A useful choice exists, with conditional efficiency evidence</h2>
<p>Always-direct remains the best fixed choice: mean fuel {h["mean_fuel_kg"]["direct"]:.2f} kg, versus {h["mean_fuel_kg"]["support_short"]:.2f} for always-support and {h["mean_fuel_kg"]["brake_long"]:.2f} for always-brake. Intermediate goals are optional.</p>
<p>In hindsight, direct wins {counts["direct"]}/60 states, support {counts["support_short"]}/60 and braking {counts["brake_long"]}/60. Choosing each state's best captured result would average {h["hindsight_mean_fuel_kg"]:.2f} kg, a {h["hindsight_mean_saving_vs_direct_kg"]:.2f} kg ({100*h["hindsight_saving_fraction"]:.2f}%) saving. There are {h["states_with_more_than_1kg_headroom"]} states with headroom above the predeclared 1 kg descriptive margin; maximum saving is {h["maximum_headroom_kg"]:.2f} kg.</p>
<p><img src="conditional-fuel-headroom.svg" alt="Fuel saving or extra cost of each arrival choice relative to the first-feasible direct selector, for all 60 confirmation states" style="max-width:100%;height:auto"></p>
<p>This is conditional headroom against the unchanged first-feasible landing selector. Hindsight chooses using all outcomes and is not an executable learned policy. A stronger direct timing search could reduce that advantage. No optimized-efficiency or RL generalization claim follows.</p>
<p><strong>Next supported experiment:</strong> a bounded three-choice planner (direct, support goal, braking goal) over this frozen controller. Use new training/validation/test inputs; the current confirmation is now exposed. Keep the development-selected constant baseline and distinguish system improvement under the fixed selector from efficiency against stronger timing baselines.</p></section>
<section><h2>Evidence and cost</h2><p>{audit["captures"]} complete captures, {audit["verified_landings"]} verified target-landing captures, {audit["prefix_proofs"]} exact retained-prefix proofs. The read-only audit independently checks source/input hashes, first-resolution predicates and ticks, original handoff state, all proofs and exact measured work. Maintained development gate: {gate["completed_checks"]} checks passed.</p>
<p>{a["resources"]["steps"]:,} measured native/model/forecast/replay steps; one native worker and zero optimizer steps. Phase wall times total {a["resources"]["phase_wall_seconds"]:.2f} s; build, implementation and review are separate. Per-update CPU cost is uninstrumented.</p>
<p><a href="http://starship:8000/arrival-actuation/loop-20261024T021508Z/report-delivery/report.html">Previous actuator calibration and mixed qualification</a></p></section>
<section><h2>Compare choices on one state</h2><label>Original state <select id="composition-state" style="display:block;width:680px;max-width:100%"></select></label><div style="overflow:auto"><table><thead><tr><th>Choice / executor</th><th>Arrival</th><th>Landing evidence</th><th>Fuel kg</th><th>Time s</th><th>Flight</th></tr></thead><tbody id="composition-results"></tbody></table></div></section>
<script>(()=>{{const rows={payload};const s=document.getElementById('composition-state'),b=document.getElementById('composition-results');for(const id of [...new Set(rows.map(r=>r.state_id))]){{const o=document.createElement('option');o.value=id;o.textContent=id;s.append(o)}}function render(){{b.replaceChildren();for(const r of rows.filter(r=>r.state_id===s.value)){{const tr=document.createElement('tr');for(const value of [r.choice+' / '+r.mode,r.result.arrival_status,r.proof?(r.proof.verified_target_landing?'verified landing':'verified non-landing'):(r.result.success?'Summary landing':'Summary miss'),r.result.fuel_used_kg.toFixed(2),r.result.stop_s.toFixed(2)]){{const td=document.createElement('td');td.textContent=value;tr.append(td)}}const td=document.createElement('td');if(r.href){{const link=document.createElement('a');link.href=r.href;link.textContent='Rich flight';td.append(link)}}else td.textContent='Summary only';tr.append(td);b.append(tr)}}}}s.addEventListener('change',render);s.value=rows.find(r=>r.phase==='confirmation').state_id;render()}})();</script>'''


def main():
    p=argparse.ArgumentParser();p.add_argument("root",type=Path);p.add_argument("output",type=Path);args=p.parse_args()
    root,output=args.root.resolve(),args.output.resolve()
    # Fail before creating report files if the separate plotting environment is missing.
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    a,audit,gate=read(root/'analysis.json'),read(root/'evidence-audit.json'),read(root/'developer-gate-final.json')
    assert audit['passed'] and gate['complete'] and gate['completed_checks']==19
    results={phase:read(root/phase/'result.json') for phase in PHASES}
    captures=[dict(c,phase=phase) for phase,d in results.items() for c in d['captures']]
    lookup={(c['case'],c['mode']):(i,c) for i,c in enumerate(captures)}
    rows=[]
    for phase,d in results.items():
        for row in d['rows']:
            case=read(row['case_path']);scenario=read(case['scenario_path']);hit=lookup.get((row['case'],row['mode']))
            rows.append(dict(row,phase=phase,state_id=case.get('state_id',scenario['id']),choice=case.get('goal_choice',case.get('choice','witness_goal')),
                proof=hit[1] if hit else None,href=f'flight-{hit[0]}.html' if hit else None))
    rows=list({(r['case'],r['mode']):r for r in rows}.values())
    spec={'title':'Arrival and landing qualification with the existing corrected controller',
        'subtitle':'120/120 fresh composites · 60/60 direct controls · unchanged Poly Landing · established mission tree',
        'summary':'Bounded local goal execution and complete landing pass; conditional choice headroom is separate from RL qualification.',
        'grouped':False,'focus_flat':True,'max_bytes':1024**3,'overview_html':overview(a,audit,gate,rows),
        'rows':[{'capture':c['capture'],'population':c['phase']+' · '+next(r['population'] for r in rows if r['case']==c['case'] and r['mode']==c['mode']),
            'label':c['phase']+' · '+read(next(r['case_path'] for r in rows if r['case']==c['case'] and r['mode']==c['mode'])).get('goal_choice','direct' if read(Path(c['capture'])/'decisions.json')['arrival'] is None else 'arrival')+' · '+c['mode'],
            'context_html':context(read(Path(c['capture'])/'decisions.json'),c['mode'],c['phase'])} for c in captures]}
    write_new(output.parent/(output.name+'-spec.json'),spec);native.render_report(encoded(spec),str(output))
    fig,ax=plt.subplots(figsize=(10,4.8),layout='constrained')
    values=a['headroom']['case_outcomes']
    for choice,color,label in [('support_short','#137a68','Support short'),('brake_long','#bc632c','Brake long')]:
        ax.plot(range(1,61),[v['choices']['direct']['fuel_used_kg']-v['choices'][choice]['fuel_used_kg'] for v in values],'.-',color=color,label=label,linewidth=1,markersize=4)
    ax.axhline(0,color='#555',linewidth=.8)
    for boundary in [15.5,30.5,45.5]:ax.axvline(boundary,color='#aaa',linestyle=':',linewidth=.8)
    ax.set(xlabel='Confirmation state index (vertical / lateral / braking / attitude preparation)',ylabel='Fuel saving versus first-feasible direct (kg)')
    ax.grid(axis='y',alpha=.2);ax.legend();fig.savefig(output/'conditional-fuel-headroom.svg');plt.close(fig)
    write_new(output/'presentation-receipt.json',{'renderer_sha256':sha(__file__),'analysis_sha256':sha(root/'analysis.json'),
        'audit_sha256':sha(root/'evidence-audit.json'),'gate_sha256':sha(root/'developer-gate-final.json'),'matplotlib_version':matplotlib.__version__,
        'rich_captures':len(captures),'comparison_states':len({r['state_id'] for r in rows})})


if __name__=='__main__':main()
