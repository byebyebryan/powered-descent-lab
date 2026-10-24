"""Input-only development/confirmation populations for two native goal recipes."""
import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
from composition_study import Budget, encoded, native, read, sha, write_new

FAMILIES = ["vertical", "lateral", "braking", "attitude_preparation"]
CHOICES = ["support_short", "brake_long"]


def uniform(key, label):
    digest = hashlib.sha256((key + "/" + label).encode()).digest()
    return int.from_bytes(digest[:8], "big") / 2**64


def proposal(anchor, cohort, seed, family_index, sink_index, replicate):
    family = FAMILIES[family_index]
    key = f"arrival-composition-input-v1/{cohort}/{seed}/{family}/{sink_index}/{replicate}"
    u = lambda label: uniform(key, label)
    direction = 1 if u("direction") >= .5 else -1
    sink = [15., 30., 45.][sink_index] + 4*u("sink")-2
    height = 725+150*u("height")
    lateral = 0 if family == "vertical" else (40 if family == "braking" else 25)+6*u("lateral")-3
    x = 0 if family == "vertical" else -direction*(310+80*u("x"))
    attitude = (direction*.25 if family == "attitude_preparation" else 0)+.08*u("attitude")-.04
    index = family_index*15+sink_index*5+replicate
    s = copy.deepcopy(anchor)
    s.update(id=f"composition-{cohort}-{index:03}", name=f"{cohort} {family} sink-band {sink_index} original {replicate}",
        description="Input-only flat airborne composition; no outcome admission or replacements", seed=seed+index,
        tags=["arrival_composition_v1", "flat", cohort, family], metadata={"family":family,"cohort":cohort,
        "generator":"arrival-composition-input-v1","sink_band":str(sink_index),"replicate":str(replicate),"direction":str(direction)})
    s["sim"].update(max_time_s=90.,sample_hz=120)
    s["mission"].pop("transfer_route",None)
    s["world"]["terrain"]["points_m"]=[{"x":-4000,"y":0},{"x":4000,"y":0}]
    s["initial_state"]={"position_m":{"x":x,"y":height},"velocity_mps":{"x":direction*lateral,"y":-sink},
        "attitude_rad":attitude,"angular_rate_radps":0.}
    recipes={}
    for choice in CHOICES:
        duration=(5.5+u("short_horizon")) if choice=="support_short" else (7.+u("long_horizon"))
        ticks=2*round(duration*60)
        powered_s=(ticks-(60 if family=="attitude_preparation" else 0))/120
        ax=0. if choice=="support_short" else -.35*direction*lateral/powered_s
        recipes[choice]={"physics_ticks":ticks,"prepare_ticks":60 if family=="attitude_preparation" else 0,
            "thrust_accel_mps2":{"x":ax,"y":9.81}}
    return s,recipes


def main():
    p=argparse.ArgumentParser();p.add_argument("allocation",type=Path);args=p.parse_args()
    a=read(args.allocation);root=Path(a["output"]);root.mkdir();b=Budget(root,a)
    if sha(native.__file__)!=a["native_sha256"] or sha(__file__)!=a["generator_sha256"]:
        raise RuntimeError("frozen input generator or native library differs")
    anchor=read(a["anchor_path"])
    if sha(a["anchor_path"])!=a["anchor_sha256"]:raise RuntimeError("anchor differs")
    entries,exclusions=[] ,[]
    for family_index in range(4):
        for sink_index in range(3):
            for replicate in range(a["replicates"]):
                s,recipes=proposal(anchor,a["cohort"],a["seed"],family_index,sink_index,replicate)
                scenario_path=root/"scenarios"/(s["id"]+".json");write_new(scenario_path,s)
                base={"state_id":s["id"],"population":a["cohort"]+"_"+FAMILIES[family_index],
                    "scenario_path":str(scenario_path),"scenario_sha256":sha(scenario_path),"generator_sha256":sha(__file__)}
                direct={**base,"id":s["id"]+"-direct","choice":"direct"}
                path=root/"cases"/(direct["id"]+".json");write_new(path,direct)
                entries.append({"id":direct["id"],"path":str(path),"sha256":sha(path),"modes":["native_pair_v2"]})
                for choice,recipe in recipes.items():
                    g=json.loads(native.generate_witness(encoded(s),encoded(recipe)))
                    b.counts["native_steps"]+=g["native_steps"];b.counts["construction_steps"]=b.counts.get("construction_steps",0)+g["native_steps"];b.check()
                    generated_path=root/"construction"/(s["id"]+"-"+choice+".json");write_new(generated_path,g)
                    if not g["admitted"]:
                        exclusions.append({"state_id":s["id"],"choice":choice,"construction":str(generated_path),"reason":g.get("reason","unchanged first-window predicate failed")});continue
                    case={**base,"id":s["id"]+"-"+choice,"goal_choice":choice,"recipe":recipe,
                        "construction_path":str(generated_path),"program":{"arrival":g["request"],"land":True}}
                    path=root/"cases"/(case["id"]+".json");write_new(path,case)
                    entries.append({"id":case["id"],"path":str(path),"sha256":sha(path),"modes":a["arrival_modes"]})
    write_new(root/"result.json",{"cohort":a["cohort"],"generator_path":str(Path(__file__).resolve()),"generator_sha256":sha(__file__),
        "allocation_sha256":sha(args.allocation),"cases":entries,"exclusions":exclusions,"resources":b.receipt()})
    print(json.dumps({"cohort":a["cohort"],"states":4*3*a["replicates"],"programs":len(entries),"exclusions":len(exclusions),"resources":b.receipt()}))


if __name__=="__main__":main()
