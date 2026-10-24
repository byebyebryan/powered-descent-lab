"""Render the complete crossed comparison using maintained rich report views."""
import argparse
import html
import json
import math
from pathlib import Path

import pd_coast_native as native

FIRST, FUEL = "first_feasible_v1", "minimum_fuel_grid_v1"
CHOICES = ["direct", "support_short", "brake_long"]
NAMES = {FIRST: "First feasible", FUEL: "Fuel-ranked bank"}


def read(path):
    return json.loads(Path(path).read_text())


def write_new(path, value):
    with Path(path).open("x") as f:json.dump(value, f, indent=2, allow_nan=False);f.write("\n")


def sha(path):
    import hashlib
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def context(info, mode, phase):
    a, s = info["arrival"], info["landing_search"]
    text = f'<section><h2>Arrival and landing execution</h2><p>{html.escape(phase)} · {html.escape(NAMES[mode])}. '
    if a:
        o = a["resolution"]
        text += f'Arrival <strong>{html.escape(a["status"])}</strong> at {o["sim_time_s"]:.3f} s. Actual handoff fuel {o["fuel_kg"]:.2f} kg; attitude {math.degrees(o["attitude_rad"]):.2f}°, angular rate {math.degrees(o["angular_rate_radps"]):.2f}°/s. '
        text += 'The controller and goal are unchanged; continuation starts from this actual state, preserving clocks and fuel.</p>'
    else:
        text += 'Direct landing has no intermediate instruction.</p>'
    text += f'<p>Selected coast/preparation {s["chosen_coast_ticks"]/120:.3f} s, followed by {s["chosen_duration_ticks"]/120:.3f} s Poly burn reference. '
    text += ('Complete native forecast enumeration of the unchanged timing bank; minimum predicted fuel among stable target contacts. The captured native flight verifies that selection.' if mode == FUEL else 'First feasible pair in the original fixed grid. This retained flight is exactly reproduced by the current default profile.')+'</p>'
    text += '<p>The supplied goal horizon constructs a position/velocity instruction; arrival still has its original absolute 30 s deadline within the 90 s mission. Requested goals are amber diamonds. Preview axes use equal metre scales shared across this physical state.</p>'
    text += '<p>Incoming contact above is measured before native landing resets final velocity. Post-contact zero velocity is not a touchdown-speed measurement. Per-update latency is uninstrumented; aggregate process CPU and forecast work are reported separately.</p></section>'
    return text


def overview(a, audit, rows):
    old, new = a["timing"][FIRST], a["timing"][FUEL]
    table = ''.join(f'<tr><td>{html.escape(c.replace("_", " "))}</td><td>{old["mean_fuel_kg"][c]:.2f}</td><td>{new["mean_fuel_kg"][c]:.2f}</td></tr>' for c in CHOICES)
    payload = json.dumps(rows, separators=(',', ':'), allow_nan=False).replace('<', '\\u003c')
    return f'''<section><h2>Timing dominates the apparent intermediate-goal benefit</h2>
<p><strong>Fuel-ranked direct landing averages {new["mean_fuel_kg"]["direct"]:.2f} kg instead of {old["mean_fuel_kg"]["direct"]:.2f} kg: {100*a["direct_timing_saving_fraction"]:.2f}% less fuel.</strong> Applying the same search after every choice leaves only {100*new["hindsight_saving_fraction"]:.2f}% hindsight goal-selection opportunity. The declared 1% learning-release gate fails, so this pass stops before fitting or RL training.</p>
<p>All 180 new timing executions have complete physical/mission/integrity/replay proof plus controller and goal reconstruction. Every one of the 120 intermediate-goal attempts fulfills its instruction before verified target landing; all 60 direct controls verify target landing. The current default reproduces all 180 retained decision records exactly. The 60 originals were exposed by the preceding study: this is a controlled comparison, not fresh population confirmation.</p>
<div style="overflow:auto"><table><thead><tr><th>Initial strategy</th><th>First-feasible mean fuel kg</th><th>Fuel-ranked mean fuel kg</th></tr></thead><tbody>{table}</tbody></table></div>
<p>Before timing optimization, hindsight goal choice saved {old["hindsight_mean_saving_kg"]:.2f} kg ({100*old["hindsight_saving_fraction"]:.2f}%); direct/support/braking won 32/21/7 states. With consistent improved continuation, the saving is {new["hindsight_mean_saving_kg"]:.2f} kg ({100*new["hindsight_saving_fraction"]:.2f}%), with direct/support/braking winning 53/6/1 states. Seven states save more than 1 kg; the largest saving is {new["hindsight_saving_range_kg"][1]:.2f} kg. These are hindsight comparisons, not an executable learned policy.</p>
<p><img src="timing-goal-comparison.svg" alt="Mean fuel by initial strategy under both timing selectors, and remaining per-state hindsight goal benefit" style="max-width:100%;height:auto"></p></section>
<section><h2>What changed and what remains bounded</h2>
<p>The new opt-in selector checks the same seven coast times (0, 0.5, 1, 2, 3, 4, 6 s) and nine burn durations (6, 8, 10, 12, 16, 20, 24, 30, 36 s). It preserves native attitude preparation, minimum thrust, fuel depletion, contact criteria and Poly arithmetic. It evaluates every pair and ranks feasible native contact forecasts by fuel, using grid order for exact ties. This is best within a finite bank, not a global optimum. Default behavior is unchanged.</p>
<p>Support and braking receive the same improved search after their first actual arrival resolution. Local guidance, the 0.72 rad arrival tilt limit, 30 m goal tolerance, speed/heading bounds and original deadlines remain fixed. The goal recipes construct a restricted family of feasible position/velocity goals; this does not establish arbitrary state control or repeated retargeting.</p>
<p>The exposed input envelope remains height 725–875 m, horizontal position zero or 310–390 m from the pad, lateral speed zero/22–28/37–43 m/s, sink 13–47 m/s and modest initial attitude. Vehicle and fuel are fixed. Every direct case already lands, so this comparison establishes efficiency changes rather than an expanded landing envelope. Earlier shallow-entry, projection/minimum-thrust and recovery failures remain in their original diagnostic report.</p></section>
<section><h2>Evidence, cost and the stop decision</h2>
<p>{audit["captures"]} new complete captures include six repeated cost-pilot flights and all 180 new comparison choices. All {audit["captures"]} verify target landing; {audit["prefix_proofs"]} retained-prefix checks preserve old native observations through first arrival and commands before it. The mission tree below also includes all 180 sealed original choices, for 366 rich occurrences on 60 comparison originals. Pilot repeats are labelled separately and never increase the population denominator.</p>
<p>Total charged work: {audit["steps"]:,} native/model/forecast/replay steps, including goal reconstruction. There are {audit["reconstructions"]} controller/goal reconstructions, one native worker, zero model fits and zero RL gradient updates. Measured campaign phases total {a["resources"]["phase_seconds"]:.2f} s; build, implementation and review are separate. New timing uses {a["costs"][FUEL]["forecast_steps"]:,} forecast steps versus {a["costs"][FIRST]["forecast_steps"]:,}; arrival model work is identical. Aggregate episode process CPU is {a["costs"][FUEL]["episode_process_cpu_seconds"]:.2f} versus {a["costs"][FIRST]["episode_process_cpu_seconds"]:.2f} s. This is not a per-update real-time qualification or isolated latency benchmark.</p>
<p>The fixed release rule required every new attempt to succeed, at least 1% mean hindsight saving and at least six states saving more than 1 kg. Safety/instruction and state-count conditions pass; mean opportunity is only {100*new["hindsight_saving_fraction"]:.2f}%. No training, validation or new final-test population is generated. All earlier outcomes and default selection remain preserved.</p>
<p>The next decision is whether to improve or amortize timing search, or study a separately defined envelope where approach changes add capability. The present result does not justify training this three-choice menu for the earlier apparent 5% benefit.</p>
<p><a href="http://starship:8000/arrival-composition/loop-20261024T051904Z/report-flight-frame/report.html">Previous composition report and retained diagnostics</a></p></section>
<section><h2>Compare all six executions on one original</h2><label>Original state <select id="timing-state" style="display:block;width:680px;max-width:100%"></select></label>
<div style="overflow:auto"><table><thead><tr><th>Strategy / timing</th><th>Arrival</th><th>Landing</th><th>Fuel kg</th><th>Coast s</th><th>Burn s</th><th>Flight</th></tr></thead><tbody id="timing-results"></tbody></table></div></section>
<script>(()=>{{const rows={payload};const select=document.getElementById('timing-state'),body=document.getElementById('timing-results');for(const id of [...new Set(rows.map(r=>r.state_id))]){{const o=document.createElement('option');o.value=id;o.textContent=id;select.append(o)}}function render(){{body.replaceChildren();for(const r of rows.filter(r=>r.state_id===select.value)){{const tr=document.createElement('tr');for(const value of [r.choice+' / '+r.timing_name,r.result.arrival_status,'verified target landing',r.result.fuel_used_kg.toFixed(2),(r.result.coast_ticks/120).toFixed(2),(r.result.duration_ticks/120).toFixed(2)]){{const td=document.createElement('td');td.textContent=value;tr.append(td)}}const td=document.createElement('td'),link=document.createElement('a');link.href=r.href;link.textContent='Rich flight';td.append(link);tr.append(td);body.append(tr)}}}}select.addEventListener('change',render);render()}})();</script>'''


def main():
    p = argparse.ArgumentParser();p.add_argument("root", type=Path);p.add_argument("output", type=Path);args = p.parse_args()
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    root, output = args.root.resolve(), args.output.resolve()
    a, audit = read(root/'analysis-final.json'), read(root/'evidence-audit.json')
    assert audit['passed'] and not a['learning_release_passed']
    comparison = read(root/'comparison/result.json')
    new = {c['case']: c for phase in ['comparison', 'proof-completion'] for c in read(root/phase/'result.json')['captures']}
    assert len(new) == 180
    retained = Path(read(root/'comparison-allocation.json')['retained_root'])
    specs, rows = [], []
    for row in sorted(comparison['rows'], key=lambda r: (r['state_id'], CHOICES.index(r['choice']), [FIRST, FUEL].index(r['mode']))):
        folder = retained/'confirmation/captures'/(row['case']+'-native_pair_v2') if row['mode'] == FIRST else Path(new[row['case']]['capture'])
        family = row['population'].removeprefix('confirmation_').replace('_', ' ')
        specs.append({'capture': str(folder), 'population': 'Exposed comparison · '+family,
            'label': row['choice'].replace('_', ' ')+' · '+NAMES[row['mode']],
            'context_html': context(read(folder/'decisions.json'), row['mode'], 'Exposed paired comparison')})
        rows.append({**row, 'timing_name': NAMES[row['mode']], 'href': f'flight-{len(specs)-1}.html'})
    for c in read(root/'pilot/result.json')['captures']:
        folder = Path(c['capture'])
        specs.append({'capture': str(folder), 'population': 'Repeated cost pilot',
            'label': 'Cost pilot · '+c['case']+' · '+NAMES[c['mode']],
            'context_html': context(read(folder/'decisions.json'), c['mode'], 'Repeated cost pilot; not a new original')})
    spec = {'title': 'Timing and intermediate-goal comparison', 'subtitle': 'Complete 60-state crossed comparison · 366 rich flights · learning gate stopped',
        'summary': 'Improved timing saves 20.45% direct fuel; remaining goal-choice opportunity is 0.21%.',
        'grouped': False, 'focus_flat': True, 'max_bytes': 2*1024**3,
        'overview_html': overview(a, audit, rows), 'rows': specs}
    write_new(root/'report-spec.json', spec)
    native.render_report(json.dumps(spec, allow_nan=False), str(output))
    fig, axes = plt.subplots(2, 1, figsize=(10, 8), layout='constrained')
    positions = list(range(3))
    for offset, mode, color in [(-.18, FIRST, '#8f9bab'), (.18, FUEL, '#147d75')]:
        values = [a['timing'][mode]['mean_fuel_kg'][c] for c in CHOICES]
        bars = axes[0].bar([x+offset for x in positions], values, width=.36, label=NAMES[mode], color=color)
        axes[0].bar_label(bars, fmt='%.1f', padding=3)
    axes[0].set(xticks=positions, xticklabels=['Direct', 'Support short', 'Brake long'], ylabel='Mean mission fuel (kg)', ylim=(0, 900))
    axes[0].legend();axes[0].grid(axis='y', alpha=.2)
    by_state = {}
    for row in comparison['rows']:by_state.setdefault(row['state_id'], {}).setdefault(row['mode'], {})[row['choice']] = row['result']['fuel_used_kg']
    for mode, color in [(FIRST, '#8f9bab'), (FUEL, '#147d75')]:
        values = [v[mode]['direct']-min(v[mode].values()) for _, v in sorted(by_state.items())]
        axes[1].plot(range(60), values, '.-', color=color, label=NAMES[mode], linewidth=1)
    for x in [14.5, 29.5, 44.5]:axes[1].axvline(x, color='#aaa', linestyle=':', linewidth=.8)
    axes[1].set(xlabel='Exposed state index · vertical / lateral / braking / attitude preparation', ylabel='Hindsight goal saving over direct (kg)')
    axes[1].legend();axes[1].grid(axis='y', alpha=.2)
    fig.savefig(output/'timing-goal-comparison.svg');plt.close(fig)
    write_new(output/'presentation-receipt.json', {'renderer_sha256': sha(__file__), 'analysis_sha256': sha(root/'analysis-final.json'),
        'audit_sha256': sha(root/'evidence-audit.json'), 'matplotlib_version': matplotlib.__version__,
        'rich_captures': len(specs), 'comparison_states': 60, 'comparison_choices_per_state': 6,
        'retained_rich_flights': 180, 'new_comparison_rich_flights': 180, 'pilot_rich_flights': 6})
    print(json.dumps({'output': str(output), 'rich_captures': len(specs), 'comparison_states': 60}))


if __name__ == '__main__':
    main()
