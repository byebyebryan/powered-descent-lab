"""Unchanged ballistic candidate on the authenticated original 1k worlds.

Diagnostic only. No tuning, retries, snapshot restore or accepted-site writes.
Each native attempt owns its full flight, command replay and decision repeat.
"""

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import json
import math
from pathlib import Path
import subprocess
import time

import diagnostics as d
import study
import survey

PLAN = survey.HERE / "ballistic_feedback_sweep_plan.json"
PLAN_SHA256 = "3eeb16e183141efa8051530ab61bc8e0591932ce5856a8d172b642265515e529"
REPLAN_PLAN = survey.HERE / "ballistic_replan_sweep_plan.json"
REPLAN_PLAN_SHA256 = "1a6c50ba80f13d9effdd0d105c19ddde65487f82c46c691786b0255aff6b0c96"
FINAL_REPLAN_PLAN = survey.HERE / "ballistic_replan_final_sweep_plan.json"
FINAL_REPLAN_PLAN_SHA256 = "af10fe08ce6926def310d6bbd22cb616d25aaaaad43209969829ff19394d6f7c"
TERRAIN_PLAN = survey.HERE / "ballistic_terrain_correction_sweep_plan.json"
TERRAIN_PLAN_SHA256 = "7e10a214b2a4b135718be5e79f8da94512c517a7c96688750cf5ed5df9a5c1f0"
COAST_PLAN = survey.HERE / "ballistic_coast_terminal_sweep_plan.json"
COAST_PLAN_SHA256 = "d7b29cd8c18a82be4eb0d1303e2082785b6439ea4a73d85267b4dab9ab55c186"
TERRAIN_CANDIDATE = "ballistic_feedback_v3_terrain_correction"
WAYPOINT_CANDIDATE = "ballistic_feedback_v4_waypoint_clearance"
RIDGE_CANDIDATE = "ballistic_feedback_v5_ridge_waypoint"
ENTRY_CANDIDATES = {name: f"ballistic_feedback_v6_waypoint_{name}" for name in ("effort", "recovery", "combined")}
LOCAL_CANDIDATES = {name: f"ballistic_feedback_v7_{name.replace('-', '_')}" for name in
                    ("local-height", "early-target", "local-height-early-target")}
LANDING_CANDIDATES = {name: f"ballistic_feedback_v8_{name.replace('-', '_')}" for name in
                      ("landing-duration", "early-target-landing-duration")}
COUNTDOWN_CANDIDATES = {name: f"ballistic_feedback_v9_{name.replace('-', '_')}" for name in
                        ("landing-countdown", "early-target-landing-countdown")}
COAST_CANDIDATE = "ballistic_feedback_v10_coast_terminal"
COAST_CANDIDATES = {"coast-terminal": COAST_CANDIDATE}
BRAKING_CANDIDATE = "ballistic_feedback_v11_landing_braking_guard"
BRAKING_CANDIDATES = {"landing-braking-guard": BRAKING_CANDIDATE}
CENTERING_CANDIDATE = "ballistic_feedback_v12_landing_body_centering"
CENTERING_CANDIDATES = {"landing-body-centering": CENTERING_CANDIDATE}
COORDINATION_CANDIDATE = "ballistic_feedback_v13_terminal_coordination"
COORDINATION_CANDIDATES = {"terminal-coordination": COORDINATION_CANDIDATE}
MECHANICS_CANDIDATES = {name: f"ballistic_feedback_v14_{name.replace('-', '_')}" for name in
                       ("exit-consistency", "piecewise-early-target", "recovery-lead", "mechanics-combined")}
FINITE_CANDIDATES = {name: f"ballistic_feedback_v15_{name.replace('-', '_')}" for name in
                    ("finite-correction-probe", "finite-correction")}
PHASE_CANDIDATES = {name: f"ballistic_feedback_v16_{name.replace('-', '_')}" for name in
                    ("transition-probe", "coast-transition", "terminal-takeover", "pad-clearance", "phase-transitions")}
FINITE_FAMILY = {**FINITE_CANDIDATES, **PHASE_CANDIDATES}
MECHANICS_FAMILY = {**MECHANICS_CANDIDATES, **FINITE_FAMILY}
LANDING_FAMILY = {**LANDING_CANDIDATES, **COUNTDOWN_CANDIDATES, **COAST_CANDIDATES, **BRAKING_CANDIDATES, **CENTERING_CANDIDATES, **COORDINATION_CANDIDATES, **MECHANICS_FAMILY}
LOCAL_FAMILY = {**LOCAL_CANDIDATES, **LANDING_FAMILY}
RIDGE_FAMILY = (RIDGE_CANDIDATE, *ENTRY_CANDIDATES.values(), *LOCAL_FAMILY.values())


def contract(path=PLAN):
    digest = study.digest(path.read_bytes())
    replan = digest == REPLAN_PLAN_SHA256
    final_replan = digest == FINAL_REPLAN_PLAN_SHA256
    terrain = digest == TERRAIN_PLAN_SHA256
    coast = digest == COAST_PLAN_SHA256
    if digest != PLAN_SHA256 and not replan and not final_replan and not terrain and not coast:
        raise ValueError("changed frozen diagnostic contract")
    plan = d.read(path)
    canonical = COAST_PLAN if coast else TERRAIN_PLAN if terrain else FINAL_REPLAN_PLAN if final_replan else REPLAN_PLAN if replan else PLAN
    if (plan != d.read(canonical) or plan["schema"] != "pd-lab.ballistic-feedback-sweep-plan.v1"
            or [plan[k] for k in ("primary_cases", "maximum_measured_attempts", "correction_cap", "workers",
                                  "case_wall_limit_s", "campaign_wall_limit_s", "retries", "tuning", "publication")]
            != [1000, 1002, 24, 4, 60, 7200, 0, False, False]
            or plan["repeat_indices"] != ([84, 715] if coast else [34, 55] if terrain else [308, 349] if final_replan else [55, 142] if replan else [0, 715])):
        raise ValueError("changed diagnostic contract")
    if replan and plan.get("candidate_id") != "ballistic_feedback_v2_replan":
        raise ValueError("wrong replan candidate")
    if final_replan and plan.get("candidate_id") != "ballistic_feedback_v2_replan_r2":
        raise ValueError("wrong final replan candidate")
    if terrain and plan.get("candidate_id") != TERRAIN_CANDIDATE:
        raise ValueError("wrong terrain correction candidate")
    if coast and (plan.get("candidate_id") != COAST_CANDIDATE or plan.get("waypoint_experiment") != "coast-terminal"):
        raise ValueError("wrong coast-terminal candidate")
    return plan


def source_state(binary):
    state = survey.source_state(binary)
    extra = ["pd-report/src/planning_cycles.js", "fixtures/reports/report_navigation.json"]
    state["files"].update({p: study.digest((survey.REPO / p).read_bytes()) for p in extra})
    return state


def protected_state(plan):
    state = survey.protected_state()
    baseline = survey.REPO / plan["baseline_capture"]
    for path in [baseline / "receipt.json", baseline / "early-exit-sweep.json",
                 survey.REPO / "fixtures/reports/report_navigation.json",
                 survey.REPO / "target/release/pd-eval"]:
        state[str(path.relative_to(survey.REPO))] = study.digest(path.read_bytes())
    # Existing navigation and published candidate review remain untouched.
    for name in ["outputs/reports/index.html", "outputs/reports/eval/index.html", "outputs/index.html"]:
        path = survey.REPO / name
        if path.is_file():
            state[name] = study.digest(path.read_bytes())
    if "previous_capture" in plan:
        previous = survey.REPO / plan["previous_capture"]
        for name in ["receipt.json", "ballistic-feedback-sweep.json", "index.html"]:
            path = previous / name
            state[str(path.relative_to(survey.REPO))] = study.digest(path.read_bytes())
    if "gate_panel_capture" in plan:
        gate_panel = survey.REPO / plan["gate_panel_capture"]
        for name in ("receipt.json", "ballistic-feedback-sweep.json", "manifest.json"):
            path = gate_panel / name
            state[str(path.relative_to(survey.REPO))] = study.digest(path.read_bytes())
    return state


def previous_results(plan, root=None):
    """Authenticate the prior candidate independently of the old 817 baseline."""
    if "previous_capture" not in plan:
        return None
    if root is None:
        previous = survey.REPO / plan["previous_capture"]
        receipt = (previous / "receipt.json").read_bytes()
        data = (previous / "ballistic-feedback-sweep.json").read_bytes()
    else:
        receipt = (root / "previous-receipt.json").read_bytes()
        data = (root / "previous-results.json").read_bytes()
    if (study.digest(receipt) != plan["previous_receipt_sha256"]
            or study.digest(data) != plan["previous_results_sha256"]
            or json.loads(receipt)["files"].get("ballistic-feedback-sweep.json") != study.digest(data)):
        raise ValueError("prior candidate receipt/result differs")
    saved = json.loads(data)
    rows = [r for r in saved["rows"] if r["cohort"] == "random"]
    if (saved["stopped_reason"] is not None or len(rows) != 1000
            or any(r["status"] != "recorded" for r in rows)
            or [r["case_id"] for r in rows] != [f"random-{i:03}" for i in range(1000)]):
        raise ValueError("incomplete prior candidate population")
    return rows


def baseline(plan):
    root = survey.REPO / plan["baseline_capture"]
    receipt = (root / "receipt.json").read_bytes()
    if study.digest(receipt) != plan["baseline_receipt_sha256"]:
        raise ValueError("baseline receipt differs")
    files = json.loads(receipt)["files"]

    def bound(relative):
        data = d.safe_file(root, relative).read_bytes()
        if study.digest(data) != files.get(relative):
            raise ValueError("unauthenticated baseline artifact: " + relative)
        return data

    data = bound("early-exit-sweep.json")
    if study.digest(data) != plan["baseline_results_sha256"]:
        raise ValueError("baseline results differ")
    rows = [r for r in json.loads(data)["rows"] if r["cohort"] == "random"]
    if len(rows) != 1000 or [r["case_id"] for r in rows] != [f"random-{i:03}" for i in range(1000)]:
        raise ValueError("wrong original world inventory")
    cases = []
    previous = previous_results(plan)
    for row in rows:
        if row["status"] != "recorded":
            raise ValueError("incomplete baseline")
        path = row["scenario_path"]
        data = bound(path)
        cases.append({"case_id": row["case_id"], "scenario_path": path, "scenario_sha256": study.digest(data),
                      "seed": row["seed"], "geometry": row["geometry"], "baseline_result": row["result"],
                      "cohort": "random", "attempt_id": row["case_id"], "status": "not_attempted"})
        if previous is not None:
            old = previous[len(cases) - 1]
            if (old["scenario_sha256"] != study.digest(data) or old["seed"] != row["seed"]
                    or old["geometry"] != row["geometry"]):
                raise ValueError("prior candidate paired inputs differ")
            cases[-1]["previous_result"] = old["result"]
    return cases, receipt, bound


def attempts(cases, plan):
    return cases + [dict(cases[i], cohort="repeat", attempt_id=f"repeat-{i:03}") for i in plan["repeat_indices"]]


def projection(feedback, candidate_id="ballistic_feedback_v1"):
    if (candidate_id not in ("ballistic_feedback_v1", "ballistic_feedback_v2_replan", "ballistic_feedback_v2_replan_r2", TERRAIN_CANDIDATE, WAYPOINT_CANDIDATE, *RIDGE_FAMILY)
            or feedback["candidate_id"] != candidate_id
            or feedback["prefix_origin"] is not None or feedback["prefix_flight_sha256"] is not None
            or feedback["terrain_neutral_diagnostic"] is not False
            or any(feedback[k] is not True for k in ("integrity_passed", "source_replay_passed", "decisions_reproduced"))
            or feedback["final_state"] != feedback["ordinary_flight"]["final_state"]):
        raise ValueError("invalid original-source verified candidate flight")
    final = feedback["final_state"]
    landed = final["physical_outcome"] == "landed_on_target" and final["mission_outcome"] == "success"
    if (feedback["stop"] == "physical_terminal") != (final["physical_outcome"] != "flying"):
        raise ValueError("terminal stop/outcome mismatch")
    first = next((r for r in feedback["refreshes"] if r["desired_arc"] is not None), None)
    result = {"verified_landing": landed, "physical_outcome": final["physical_outcome"],
            "mission_outcome": final["mission_outcome"], "planning_stop": feedback["stop"],
            "stop_group": feedback["stop"].split(":", 1)[0], "handoffs": len(feedback["handoffs"]),
            "refreshes": len(feedback["refreshes"]), "sim_time_s": final["sim_time_s"],
            "physics_step": final["physics_step"],
            "initial_candidate_arc": "not_constructed" if first is None else
            ("blocked" if first["terrain_conflict_m"] is not None else "clear")}
    if candidate_id != "ballistic_feedback_v1":
        revisions = feedback["handoff_goal_revisions"]
        accepted = [r for r in feedback["refreshes"] if r["decision"] in ("waypoint_selected", "waypoint_replaced")]
        ids = [r["goal"]["revision"] for r in accepted]
        if len(revisions) != len(feedback["handoffs"]) or len(ids) != len(set(ids)):
            raise ValueError("invalid accepted goal/handoff revision inventory")
        for number, revision in enumerate(revisions, 1):
            matches = [r for r in accepted if r["goal"]["revision"] == revision
                       and r["goal"]["number"] == number and not r["goal"]["destination"]]
            if len(matches) != 1 or matches[0]["origin"]["physics_step"] >= feedback["handoffs"][number - 1]["physics_step"]:
                raise ValueError("handoff not bound to an earlier accepted goal")
        result.update(waypoint_selections=sum(r["decision"] == "waypoint_selected" for r in accepted),
                      waypoint_replacements=sum(r["decision"] == "waypoint_replaced" for r in accepted),
                      rejected_proposals=sum(r["decision"] in ("waypoint_proposal_blocked", "waypoint_command_blocked")
                                             for r in feedback["refreshes"]))
        if candidate_id in ("ballistic_feedback_v2_replan_r2", TERRAIN_CANDIDATE, WAYPOINT_CANDIDATE, *RIDGE_FAMILY):
            result["waypoint_reacquisitions"] = sum(r["decision"] == "waypoint_reacquired" for r in feedback["refreshes"])
    if candidate_id in (TERRAIN_CANDIDATE, WAYPOINT_CANDIDATE, *RIDGE_FAMILY):
        validate_terrain_corrections(feedback)
        result.update(terrain_recovery_episodes=sum(r["decision"] == "terrain_recovery_started" for r in feedback["refreshes"]),
                      terrain_recovery_resumed=sum(r["decision"] == "terrain_recovery_resumed" for r in feedback["refreshes"]),
                      terrain_recovery_updates=sum(u["phase"] == "terrain_clearance_recovery" for u in feedback["updates"]))
    if candidate_id in (WAYPOINT_CANDIDATE, *RIDGE_FAMILY):
        validate_waypoint_repairs(feedback)
        result["waypoint_height_repairs"] = sum(bool(r.get("waypoint_height_repair")) for r in feedback["refreshes"])
    if candidate_id in RIDGE_FAMILY:
        validate_ridge_selections(feedback)
        result["ridge_waypoint_proposals"] = sum(
            (r.get("ridge_selection") or {}).get("kind") == "blocking_ridge" for r in feedback["refreshes"])
        result["local_climb_stage_proposals"] = sum(
            (r.get("ridge_selection") or {}).get("kind") == "local_climb_stage" for r in feedback["refreshes"])
    if candidate_id in (*ENTRY_CANDIDATES.values(), *LOCAL_FAMILY.values()):
        result["waypoint_acquisition_continuations"] = sum(
            r["decision"] == "waypoint_acquisition_continues" for r in feedback["refreshes"])
    if candidate_id in LOCAL_FAMILY.values():
        validate_local_waypoints(feedback)
        result["early_destination_reacquisitions"] = sum(
            r["decision"] == "destination_reacquired_before_waypoint" for r in feedback["refreshes"])
    return result


def validate_local_waypoints(feedback):
    local = feedback["candidate_id"] not in (LOCAL_CANDIDATES["early-target"], LANDING_CANDIDATES["early-target-landing-duration"],
                                             COUNTDOWN_CANDIDATES["early-target-landing-countdown"])
    early = feedback["candidate_id"] != LOCAL_CANDIDATES["local-height"]
    for r in feedback["refreshes"]:
        selection = r.get("ridge_selection")
        if selection:
            basis = selection.get("local_height")
            if bool(basis) != local:
                raise ValueError("local height mode/provenance differs")
            if basis and (not all(math.isfinite(basis[k]) for k in
                                  ("terrain_height_m", "incoming_corridor_max_m", "allowance_m"))
                          or basis["allowance_m"] <= 0
                          or (selection["crest_m"] and basis["terrain_height_m"] < selection["crest_m"]["y"])
                          or (not r.get("waypoint_height_repair") and abs(r["goal"]["position_m"]["y"]
                              - basis["terrain_height_m"] - basis["allowance_m"]) > 1e-10)):
                raise ValueError("local height seed differs")
        room = r.get("waypoint_braking_room")
        if r["decision"] in ("destination_reacquired_before_waypoint", "early_destination_obstruction"):
            previous = r.get("previous_goal") or {}
            arc = r.get("desired_arc") or {}
            if (not early or not r["goal"]["destination"] or previous.get("destination") is not False
                    or r["goal"]["revision"] <= previous["revision"] or arc.get("target_m") != r["goal"]["position_m"]
                    or r["origin"]["held_command"]["throttle_frac"] != 0.0 or not room
                    or not all(math.isfinite(v) for v in room.values()) or room["remaining_room_m"] >= 0
                    or room["required_distance_m"] <= 0
                    or any(h["physics_step"] == r["origin"]["physics_step"] for h in feedback["handoffs"])):
                raise ValueError("early destination preview is not a distinct guarded goal change")
        elif room is not None:
            raise ValueError("unexpected destination preview estimate")


def validate_ridge_selections(feedback):
    for index, refresh in enumerate(feedback["refreshes"]):
        selection = refresh.get("ridge_selection")
        if selection is None:
            if refresh.get("replan_trigger") and refresh["decision"] in (
                    "waypoint_selected", "waypoint_replaced", "waypoint_reacquired",
                    "waypoint_proposal_blocked", "waypoint_aim_construction_miss"):
                raise ValueError("local waypoint proposal has no ridge selection")
            continue
        d_m = selection["diameter_m"]
        current_x = refresh["origin"]["position_m"]["x"]
        conflict_x = selection["conflict_x_m"]
        begin, end = selection["scan_start_x_m"], selection["scan_end_x_m"]
        drop = selection["separating_drop_m"]
        goal, crest = refresh["goal"], selection["crest_m"]
        if (not all(math.isfinite(v) for v in (d_m, current_x, conflict_x, begin, end, drop))
                or d_m <= 0 or goal["destination"] or begin != max(current_x, conflict_x - d_m * .5)
                or end < begin or drop < 0
                or (refresh.get("replan_trigger") or {}).get("state", {}).get("position_m", {}).get("x") != conflict_x):
            raise ValueError("invalid ridge selection source/scan binding")
        if selection["kind"] == "blocking_ridge":
            if (crest is None or not all(math.isfinite(crest[k]) for k in ("x", "y"))
                    or not begin <= crest["x"] <= end or drop < d_m
                    or goal["position_m"]["x"] != max(crest["x"] + d_m, current_x + d_m)):
                raise ValueError("ridge waypoint does not clear its selected crest")
        elif selection["kind"] == "local_climb_stage":
            if crest is not None or drop >= d_m or goal["position_m"]["x"] != max(conflict_x, current_x) + d_m:
                raise ValueError("unresolved slope is not a local climb stage")
        else:
            raise ValueError("unknown ridge placement kind")
        if refresh.get("waypoint_height_repair"):
            if index == 0 or feedback["refreshes"][index - 1].get("ridge_selection") != selection:
                raise ValueError("height repair changed its ridge feature")


def validate_waypoint_repairs(feedback):
    for index, refresh in enumerate(feedback["refreshes"]):
        repair = refresh.get("waypoint_height_repair")
        if repair is None:
            continue
        if index == 0:
            raise ValueError("height repair has no rejected predecessor")
        previous = feedback["refreshes"][index - 1]
        old, goal = repair["from_goal"], refresh["goal"]
        arc = previous["desired_arc"]
        lift = repair["height_increase_m"]
        tolerance = repair["handoff_height_tolerance_m"]
        relative = repair["limiting_conflict"]["state"]["physics_step"] - refresh["origin"]["physics_step"]
        if (previous["decision"] != "waypoint_proposal_blocked" or previous["origin"] != refresh["origin"]
                or old != previous["goal"] or old["destination"] or goal["destination"]
                or old["number"] != goal["number"] or old["position_m"]["x"] != goal["position_m"]["x"]
                or not math.isfinite(lift) or lift <= 0 or not math.isfinite(tolerance) or tolerance <= 0
                or goal["position_m"]["y"] != old["position_m"]["y"] + lift
                or repair["checked_through_relative_tick"] != arc["steps"] + 240
                or not 1 <= relative <= repair["checked_through_relative_tick"]):
            raise ValueError("height repair is not bound to a same-state rejected waypoint")
        if repair["limiting_conflict"]["cause"] not in (
                "waypoint_incoming_body_reserve", "waypoint_ideal_continuation_reserve"):
            raise ValueError("unexpected waypoint repair constraint")


def validate_terrain_corrections(feedback):
    """Bind recovery decisions to unchanged goals and the actual command ledger."""
    active, intervals, previous_goal, finite_obstruction = None, [], None, None
    updates = {u["physics_step"]: u for u in feedback["updates"]}
    for refresh in feedback["refreshes"]:
        decision, goal = refresh["decision"], refresh["goal"]
        tick = refresh["origin"]["physics_step"]
        if (decision == "finite_destination_query"
                and feedback["candidate_id"] in FINITE_FAMILY.values()):
            # This is query evidence, not a committed route-goal change.
            continue
        query = refresh.get("finite_destination_query")
        if (decision == "ballistic_obstruction" and query
                and feedback["candidate_id"] in (FINITE_CANDIDATES["finite-correction"], *PHASE_CANDIDATES.values())):
            acquisition = query.get("acquisition") or {}
            conflict = acquisition.get("conflict")
            if (conflict is not None and acquisition.get("rejection") is not None
                    and acquisition.get("origin") == refresh["origin"]
                    and refresh.get("predicted_conflict") == conflict):
                finite_obstruction = (refresh["origin"], conflict)
        transition = refresh.get("transition_query")
        if transition and transition.get("conflict") is not None:
            if feedback["candidate_id"] in (PHASE_CANDIDATES["coast-transition"], PHASE_CANDIDATES["phase-transitions"]):
                finite_obstruction = (refresh["origin"], transition["conflict"])
        if decision in ("waypoint_selected", "waypoint_replaced", "waypoint_reacquired"):
            trigger = refresh.get("replan_trigger") or {}
            if (trigger.get("cause") != "ideal_arc_reserve"
                    and finite_obstruction != (refresh["origin"], trigger)):
                raise ValueError("command warning changed route goal")
        if decision == "terrain_recovery_started":
            if active is not None or previous_goal != goal:
                raise ValueError("recovery start changed goal or nested episodes")
            active = (tick, goal)
        if active is not None:
            if goal != active[1] or not 0 <= tick - active[0] <= 600:
                raise ValueError("recovery changed goal or exceeded episode bound")
        if decision in ("terrain_recovery_started", "terrain_recovery_command"):
            proof = refresh.get("terrain_correction")
            update = updates.get(tick)
            if (active is None or proof is None or proof["episode_start_physics_step"] != active[0]
                    or not 24 <= proof["prediction_ticks"] <= 240
                    or proof["selected_choice"] not in ("support_requested_turn", "support_current_attitude", "upright_lift", "braking_lift")
                    or proof["selected_command"]["throttle_frac"] != 1.0
                    or refresh["predicted_command"] != proof["selected_command"]
                    or update is None or update["command"] != proof["selected_command"]
                    or update["phase"] != "terrain_clearance_recovery"):
                raise ValueError("recovery decision/actual command binding differs")
        if decision == "terrain_recovery_resumed":
            if active is None:
                raise ValueError("resumed without recovery episode")
            intervals.append((active[0], tick, active[1]["revision"]))
            active = None
        previous_goal = goal
    if active is not None:
        if not 0 <= feedback["final_state"]["physics_step"] - active[0] <= 600:
            raise ValueError("unfinished recovery exceeded episode bound")
        intervals.append((active[0], feedback["final_state"]["physics_step"], active[1]["revision"]))
    for update in feedback["updates"]:
        if update["phase"] == "terrain_clearance_recovery" and not any(
                start <= update["physics_step"] < end for start, end, _ in intervals):
            raise ValueError("unbound recovery command")
    for start, end, active_revision in intervals:
        for handoff, revision in zip(feedback["handoffs"], feedback["handoff_goal_revisions"]):
            tick = handoff["physics_step"]
            # A previous leg may hand off just before next-leg recovery starts
            # at the same actual clock. It is not H of the recovering goal.
            if start < tick < end or (tick == start and revision == active_revision):
                raise ValueError("recovery invented waypoint handoff")


def validate_landing_duration(attempt, candidate_id):
    metadata = attempt.get("landing_duration")
    if candidate_id in LANDING_FAMILY.values():
        if metadata != {
                "enabled": True, "rule": "initial_vertical_balance_after_all_latest_safe_fits_fail",
                "additional_queries": 1, "burn_time_min_s": 3.0, "burn_time_max_s": 14.0,
                "shared_entry_and_live_controller": True, "ordinary_default_changed": False}:
            raise ValueError("landing duration experiment contract differs")
    elif metadata is not None and metadata.get("enabled") is not False:
        raise ValueError("landing duration enabled under an older candidate identity")


def validate_landing_countdown(attempt, candidate_id):
    metadata = attempt.get("landing_countdown")
    if candidate_id in (*COUNTDOWN_CANDIDATES.values(), COAST_CANDIDATE, BRAKING_CANDIDATE, CENTERING_CANDIDATE, COORDINATION_CANDIDATE, *MECHANICS_FAMILY.values()):
        if metadata != {
                "enabled": True, "rule": "retain_first_selected_ballistic_fallback_arrival",
                "admissions_per_landing": 1, "remaining_time_floor_s": 0.5,
                "revalidate_each_update": True, "release_on_infeasible_or_expired": True,
                "target_convention_changed": False, "ordinary_default_changed": False}:
            raise ValueError("landing countdown experiment contract differs")
    elif metadata is not None and metadata.get("enabled") is not False:
        raise ValueError("landing countdown enabled under an older candidate identity")


COAST_METADATA = {
    "enabled": True, "only_after_actual_handoff": True, "trigger": "blocked_destination_arc",
    "checkpoint": "first_crest_plus_half_body_diameter", "maximum_coast_ticks": 960,
    "terminal_preview_ticks": 240, "terminal_setup": "standalone_defaults",
    "entry_query": "existing_dynamics_then_actual_feedback_preview",
    "configured_terminal_terrain_enabled": True, "complete_landing_suffix_required": False,
    "saved_clock_used": False,
}


def validate_landing_braking_guard(attempt, candidate_id):
    metadata = attempt.get("landing_braking_guard")
    if candidate_id in (BRAKING_CANDIDATE, CENTERING_CANDIDATE, COORDINATION_CANDIDATE, *MECHANICS_FAMILY.values()):
        if metadata != {"enabled": True,
                        "rule": "existing_braking_envelope_activates_touchdown_rescue",
                        "nominal_upward_acceleration_insufficient": True,
                        "standalone_coast_terminal_changed": False,
                        "ordinary_default_changed": False, "short_command_guard_changed": False}:
            raise ValueError("landing braking guard experiment contract differs")
    elif metadata is not None and metadata.get("enabled") is not False:
        raise ValueError("landing braking guard enabled under an older identity")


def validate_landing_body_centering(attempt, candidate_id):
    metadata = attempt.get("landing_body_centering")
    if candidate_id in (CENTERING_CANDIDATE, COORDINATION_CANDIDATE, *MECHANICS_FAMILY.values()):
        if metadata != {"enabled": True, "rule": "rotated_hull_and_feet_rescue_pad_interval",
                        "reuse_outside_pad_lateral_target": True, "vertical_authority_cap_unchanged": True,
                        "standalone_coast_terminal_changed": False, "ordinary_default_changed": False,
                        "short_command_guard_changed": False}:
            raise ValueError("landing body centering experiment contract differs")
    elif metadata is not None and metadata.get("enabled") is not False:
        raise ValueError("landing body centering enabled under an older identity")


def validate_coast_terminal(attempt, feedback):
    enabled = feedback["candidate_id"] in (COAST_CANDIDATE, BRAKING_CANDIDATE, CENTERING_CANDIDATE, COORDINATION_CANDIDATE, *MECHANICS_FAMILY.values())
    metadata = attempt.get("coast_terminal")
    if enabled:
        if metadata != COAST_METADATA:
            raise ValueError("coast-terminal experiment contract differs")
    elif metadata is not None and metadata.get("enabled") is not False:
        raise ValueError("coast-terminal enabled under an older identity")
    active = None
    intervals = []
    for refresh in feedback["refreshes"]:
        decision = refresh["decision"]
        preview = refresh.get("coast_terminal")
        rejection = refresh.get("coast_terminal_rejection")
        if not decision.startswith("coast_through_") and decision != "coast_terminal_entry":
            if preview is not None or rejection is not None:
                raise ValueError("unexpected coast-terminal proof")
            continue
        origin, goal = refresh["origin"], refresh["goal"]
        tick = origin["physics_step"]
        if (not enabled or not goal["destination"] or goal["number"] != 0
                or origin["held_command"]["throttle_frac"] != 0.0
                or not any(h["physics_step"] <= tick for h in feedback["handoffs"])):
            raise ValueError("coast-terminal changed source/goal ownership")
        if decision == "coast_through_rejected":
            if not isinstance(rejection, str) or not rejection or preview is not None or active is not None:
                raise ValueError("unbound coast rejection")
            continue
        if preview is None:
            raise ValueError("missing coast-terminal preview")
        entry = preview["entry"]
        entry_tick = entry["origin"]["physics_step"]
        if (preview["diameter_m"] <= 0 or not math.isfinite(preview["diameter_m"])
                or abs(preview["clearance_x_m"] - preview["crest_m"]["x"] - preview["diameter_m"] * 0.5) > 1e-9
                or not 0 <= preview["coast_ticks"] <= 960 or preview["coast_ticks"] % 2
                or entry["origin"]["position_m"]["x"] < preview["clearance_x_m"]
                or entry["origin"]["velocity_mps"]["y"] >= 0
                or entry["origin"]["held_command"]["throttle_frac"] != 0.0
                or not isinstance(entry["configured_initial_candidate_terrain_safe"], bool)
                or not math.isfinite(entry["ballistic_miss_m"])
                or entry["end_state"]["physics_step"] - entry_tick != entry["checked_ticks"]
                or not 0 < entry["checked_ticks"] <= 240):
            raise ValueError("coast-terminal query bounds differ")
        if entry["checked_ticks"] < 240 and not (
                entry["end_state"]["physical_outcome"] == "landed_on_target"
                and entry["end_state"]["mission_outcome"] == "success"):
            raise ValueError("short terminal preview is not a target landing")
        if decision == "coast_through_selected":
            if active is not None or rejection is not None or entry_tick != tick + preview["coast_ticks"]:
                raise ValueError("coast selection clock differs")
            if entry["origin"]["fuel_kg"] != origin["fuel_kg"]:
                raise ValueError("coast query consumed fuel")
            active = (refresh, preview)
        elif decision in ("coast_terminal_entry", "coast_through_cancelled"):
            if active is None or goal != active[0]["goal"]:
                raise ValueError("coast entry/cancellation has no unchanged selected goal")
            start = active[0]["origin"]["physics_step"]
            if decision == "coast_terminal_entry":
                if (rejection is not None or entry["origin"] != origin
                        or origin != active[1]["entry"]["origin"]):
                    raise ValueError("actual terminal entry differs from coast source prediction")
            elif not isinstance(rejection, str) or not rejection:
                raise ValueError("coast cancellation has no reason")
            intervals.append((start, tick))
            active = None
        else:
            raise ValueError("unknown coast-terminal decision")
    if active is not None:
        intervals.append((active[0]["origin"]["physics_step"], feedback["final_state"]["physics_step"]))
    for start, end in intervals:
        if any(start < h["physics_step"] <= end for h in feedback["handoffs"]):
            raise ValueError("coast alternative invented H")
        updates = [u for u in feedback["updates"] if start <= u["physics_step"] < end]
        if (len(updates) * 2 != end - start or any(
                u["phase"] != "coast_through_to_terminal"
                or u["command"] != {"throttle_frac": 0.0, "target_attitude_rad": 0.0}
                for u in updates)):
            raise ValueError("actual engine-off coast differs")
    for update in feedback["updates"]:
        if update["phase"] == "coast_through_to_terminal" and not any(
                start <= update["physics_step"] < end for start, end in intervals):
            raise ValueError("unbound coast-through command")


def check_record(root, row, plan):
    output = d.safe_file(root, row["output_dir"])
    input_bytes = d.safe_file(root, row["scenario_path"]).read_bytes()
    feedback = d.read(output / "feedback.json")
    attempt = d.read(output / "attempt.json")
    if ((output / "scenario.json").read_bytes() != input_bytes
            or study.digest(input_bytes) != row["scenario_sha256"]
            or feedback["scenario_sha256"] != row["scenario_sha256"]
            or attempt["scenario_sha256"] != row["scenario_sha256"]
            or attempt["binary_sha256"] != plan["candidate_executable_sha256"]
            or attempt["content_source"]["rust_source_tree_sha256"] != plan["candidate_rust_source_sha256"]
            or attempt["report_script_sha256"] != plan["candidate_report_script_sha256"]
            or attempt["baseline_path"] is not None or attempt["terrain_neutral_diagnostic"]
            or attempt["correction_cap"] != plan["correction_cap"] or attempt["refresh_ticks"] != 24
            or d.read(output / "preflight.json")["supported"] is not True):
        raise ValueError("native input/source/policy binding differs")
    if plan.get("candidate_id", "ballistic_feedback_v1") != "ballistic_feedback_v1" and attempt.get("maximum_local_proposals") != 4:
        raise ValueError("local replan bound differs")
    if plan.get("candidate_id") in (TERRAIN_CANDIDATE, WAYPOINT_CANDIDATE, *RIDGE_FAMILY) and attempt.get("terrain_correction") != {
            "maximum_commands": 4, "maximum_episode_ticks": 600, "maximum_prediction_ticks": 240,
            "braking_tilt_rad": 0.5235987755982988, "short_command_changes_goal": False}:
        raise ValueError("terrain correction contract differs")
    if plan.get("candidate_id") in (WAYPOINT_CANDIDATE, *RIDGE_FAMILY) and attempt.get("waypoint_clearance") != {
            "repair": "fixed_x_body_envelope_height_lift", "continuation_ticks": 240,
            "numeric_height_margin_m": 1e-6, "direct_constructor_terrain_blind": True}:
        raise ValueError("waypoint height repair contract differs")
    if plan.get("candidate_id") in RIDGE_FAMILY and attempt.get("ridge_placement") != {
            "rule": "first_crest_body_diameter_drop", "valley_drop_vehicle_diameters": 1,
            "crest_offset_vehicle_diameters": 1, "unresolved_slope": "local_climb_stage",
            "landing_suffix_required": False}:
        raise ValueError("ridge placement contract differs")
    if plan.get("candidate_id") in ENTRY_CANDIDATES.values():
        mode = plan.get("waypoint_experiment")
        if ENTRY_CANDIDATES.get(mode) != plan["candidate_id"] or attempt.get("waypoint_entry") != {
                "least_added_thrust_effort": mode in ("effort", "combined"),
                "forward_crossing_recovery": mode in ("recovery", "combined"),
                "retain_safe_pending_coast": mode in ("recovery", "combined"),
                "apex_constraint": False, "destination_constructor_changed": False}:
            raise ValueError("waypoint entry ablation contract differs")
    if plan.get("candidate_id") in LOCAL_FAMILY.values():
        mode = plan.get("waypoint_experiment")
        if (LOCAL_FAMILY.get(mode) != plan["candidate_id"] or attempt.get("waypoint_entry") != {
                "least_added_thrust_effort": True, "forward_crossing_recovery": True,
                "retain_safe_pending_coast": True, "apex_constraint": False, "destination_constructor_changed": False}
                or attempt.get("local_waypoint") != {
                    "feature_local_height": mode not in ("early-target", "early-target-landing-duration", "early-target-landing-countdown"),
                    "negative_braking_room_destination_preview": mode != "local-height",
                    "preview_records_handoff": False, "destination_constructor_changed": False}):
            raise ValueError("local waypoint ablation contract differs")
    validate_landing_duration(attempt, plan.get("candidate_id", "ballistic_feedback_v1"))
    validate_landing_countdown(attempt, plan.get("candidate_id", "ballistic_feedback_v1"))
    validate_landing_braking_guard(attempt, plan.get("candidate_id", "ballistic_feedback_v1"))
    validate_landing_body_centering(attempt, plan.get("candidate_id", "ballistic_feedback_v1"))
    validate_coast_terminal(attempt, feedback)
    validate_terminal_coordination(attempt, feedback)
    validate_mechanics(attempt, feedback)
    validate_phase_transition(attempt, feedback)
    raw = d.read(output / "executed-raw.json")
    if raw.get("controller_updates", []) != feedback.get("controller_updates", []):
        raise ValueError("pre-verification terminal frame evidence differs")
    if raw.get("terrain_domain_stop") != feedback.get("terrain_domain_stop"):
        raise ValueError("pre-verification domain diagnostic differs")
    if any(raw[k] != feedback[k] for k in ("ordinary_flight", "refreshes", "handoffs", "updates", "stop")):
        raise ValueError("pre-verification execution differs")
    if plan.get("candidate_id", "ballistic_feedback_v1") != "ballistic_feedback_v1" and raw["handoff_goal_revisions"] != feedback["handoff_goal_revisions"]:
        raise ValueError("pre-verification handoff goal binding differs")
    replay = d.read(output / "command-replay.json")
    if replay["passed"] is not True or replay["final_state"] != feedback["final_state"]:
        raise ValueError("native complete-state command replay differs")
    printed = d.read(root / "logs" / (row["attempt_id"] + ".stdout"))
    expected = {"candidate_id": feedback["candidate_id"], "stop": feedback["stop"],
                "physical_outcome": feedback["final_state"]["physical_outcome"],
                "mission_outcome": feedback["final_state"]["mission_outcome"],
                "physics_step": feedback["final_state"]["physics_step"],
                "refreshes": len(feedback["refreshes"]), "handoffs": len(feedback["handoffs"]),
                "source_replay_passed": True, "integrity_passed": True, "decisions_reproduced": True}
    if printed != expected or row["exit_code"] != 0:
        raise ValueError("native stdout differs")
    if row["cohort"] == "repeat" and feedback != d.read(root / "runs" / row["case_id"] / "feedback.json"):
        raise ValueError("exact same-world complete feedback repeat differs")
    if not (output / "report.html").is_file() or not (output / "planning-cycles.json").is_file():
        raise ValueError("missing rich detail or planning views")
    return projection(feedback, plan.get("candidate_id", "ballistic_feedback_v1"))


def validate_terminal_coordination(attempt, feedback):
    enabled = feedback["candidate_id"] in (COORDINATION_CANDIDATE, *MECHANICS_FAMILY.values())
    metadata = attempt.get("terminal_coordination")
    if enabled:
        if metadata != {"enabled": True,
                        "early_braking": "preserve_nominal_lateral_acceleration_with_lift_constraint",
                        "touchdown": "body_contained_low_energy_upright_settlement",
                        "ordinary_default_changed": False, "standalone_coast_terminal_changed": False,
                        "physical_guards_changed": False}:
            raise ValueError("terminal coordination contract differs")
    elif metadata is not None and metadata.get("enabled") is not False:
        raise ValueError("terminal coordination enabled under older identity")
    diagnostic = feedback.get("terrain_domain_stop")
    domain_stop = feedback["stop"] in ("prediction_terrain_domain", "actual_terrain_domain")
    if bool(diagnostic) != domain_stop or (domain_stop and not enabled):
        raise ValueError("terrain-domain diagnostic/stop mismatch")
    if diagnostic:
        if (diagnostic["actual_state"] != feedback["final_state"]
                or feedback["final_state"]["physical_outcome"] != "flying"
                or set(diagnostic["error"]) != {"DomainOverrun"}):
            raise ValueError("domain diagnostic invented actual outcome or error")
        error = diagnostic["error"]["DomainOverrun"]
        if not (error["x_m"] < error["domain_min_x_m"] or error["x_m"] > error["domain_max_x_m"]):
            raise ValueError("domain diagnostic query is not outside terrain")
        if feedback["stop"] == "prediction_terrain_domain":
            if (diagnostic["query"] != "short_command_prediction" or diagnostic["requested_command"] is None
                    or diagnostic["query_state"]["physics_step"] <= diagnostic["query_origin"]["physics_step"]):
                raise ValueError("prediction-domain command/origin evidence differs")
        elif (diagnostic["query"] != "actual_body_reserve" or diagnostic["requested_command"] is not None
              or diagnostic["query_state"] != diagnostic["actual_state"]):
            raise ValueError("actual-domain failure mislabeled as prediction")


def validate_finite_correction(attempt, feedback):
    mode = next((m for m, candidate in FINITE_FAMILY.items()
                 if candidate == feedback["candidate_id"]), None)
    metadata = attempt.get("finite_correction")
    queries = [r for r in feedback.get("refreshes", []) if r.get("finite_destination_query")]
    if mode is None:
        if queries or (metadata is not None and metadata.get("queries_recorded") is not False):
            raise ValueError("finite query under an older identity")
        return
    if metadata != {"queries_recorded": True, "admission_enabled": mode != "finite-correction-probe",
                    "powered_prefix_plant_checked": True, "optional_refresh_ticks": 24,
                    "initial_nominal_changed": False, "waypoint_ranking_changed": False,
                    "physical_guards_changed": False}:
        raise ValueError("finite correction contract differs")
    for refresh in queries:
        query = refresh["finite_destination_query"]
        if (query["desired_arc"] != refresh["desired_arc"]
                or query["correction"] != refresh["correction"]):
            raise ValueError("finite query trajectory binding differs")
        if refresh["decision"] == "finite_destination_query":
            previous = refresh.get("previous_goal")
            if not previous or previous["destination"] or not refresh["goal"]["destination"]:
                raise ValueError("finite preview changed active-goal evidence")
        acquisition = query["acquisition"]
        if acquisition is None:
            if query["rejection"] is None:
                raise ValueError("unmeasured finite proposal claims acceptance")
            continue
        if acquisition["origin"] != refresh["origin"] or query["rejection"] != acquisition["rejection"]:
            raise ValueError("finite acquisition origin/verdict differs")
        cutoff, correction = acquisition["cutoff"], query["correction"]
        checked = acquisition["checked_powered_ticks"]
        expected = correction["burn_end_physics_step"] - refresh["origin"]["physics_step"] if correction else 0
        if not 0 <= checked <= expected or (cutoff and cutoff["physics_step"] != refresh["origin"]["physics_step"] + checked):
            raise ValueError("finite acquisition clocks differ")
        if acquisition["rejection"] is None and (cutoff is None or checked != expected
                or acquisition["coast_miss_m"] is None or acquisition["conflict"] is not None
                or acquisition["domain_stop"] is not None or acquisition["coast_domain_error"] is not None):
            raise ValueError("incomplete finite acquisition claims acceptance")


def validate_phase_transition(attempt, feedback):
    mode = next((m for m, candidate in PHASE_CANDIDATES.items() if candidate == feedback["candidate_id"]), None)
    if mode is None:
        if feedback.get("controller_updates") or any(r.get("transition_query") or r.get("recovery_common_query") for r in feedback["refreshes"]):
            raise ValueError("phase query under an older identity")
        return
    if attempt.get("phase_transition") != {
            "queries_recorded": True, "actual_coast_settling": mode in ("coast-transition", "phase-transitions"),
            "configured_terminal_takeover": mode in ("terminal-takeover", "phase-transitions"),
            "pad_reserve_command_adapter": mode in ("pad-clearance", "phase-transitions"),
            "recovery_diagnostic_only": True, "terminal_prefix_ticks": 240,
            "query_refresh_ticks": 24, "ordinary_default_changed": False, "physical_guards_changed": False}:
        raise ValueError("phase transition contract differs")

    def query_check(query, origin):
        if (query["origin"] != origin or query["kind"] not in
                ("actual_idle_rotation", "configured_terminal_prefix", "queued_turn_burn_coast")
                or not 0 < query["prediction_ticks"] <= 240
                or not 0 <= query["checked_ticks"] <= query["prediction_ticks"]):
            raise ValueError("transition origin/kind/clock differs")
        conflict = query["conflict"]
        # Queued/terminal prefixes apply the ordinary 24-tick lookahead at each
        # pair, so their first violation can follow the executed prefix horizon.
        if conflict and not 1 <= conflict["state"]["physics_step"] - origin["physics_step"] <= query["prediction_ticks"] + 24:
            raise ValueError("transition conflict outside native horizon")
        end = query["end_state"]
        if end and end["physics_step"] - origin["physics_step"] != query["checked_ticks"]:
            raise ValueError("transition end clock differs")
        if query["rejection"] is None:
            if conflict or query["domain_stop"] or end is None:
                raise ValueError("incomplete transition claims acceptance")
            if query["checked_ticks"] != query["prediction_ticks"] and not (
                    end["physical_outcome"] == "landed_on_target" and end["mission_outcome"] == "success"):
                raise ValueError("incomplete clear transition prefix")

    for refresh in feedback["refreshes"]:
        if refresh.get("transition_query"):
            query_check(refresh["transition_query"], refresh["origin"])
        acquisition = (refresh.get("finite_destination_query") or {}).get("acquisition")
        if acquisition and acquisition.get("coast_settling"):
            query_check(acquisition["coast_settling"], acquisition["cutoff"])
        common = refresh.get("recovery_common_query")
        if common:
            if common["origin"] != refresh["origin"] or not 24 <= common["prediction_ticks"] <= 240:
                raise ValueError("common recovery origin/horizon differs")
            choices = common["commands"]
            if not 1 <= len(choices) <= 4 or len({json.dumps(c["command"], sort_keys=True) for c in choices}) != len(choices):
                raise ValueError("common recovery command count differs")
            for choice in choices:
                if (not choice["checked"] or choice["prediction_ticks"] != common["prediction_ticks"]
                        or choice["command"]["throttle_frac"] != 1.0):
                    raise ValueError("recovery alternatives do not share a horizon")
                conflict = choice["conflict"]
                if conflict and not 1 <= conflict["state"]["physics_step"] - common["origin"]["physics_step"] <= common["prediction_ticks"]:
                    raise ValueError("common recovery conflict outside native horizon")
            query_check(common["queued_program"], common["origin"])
    actions = {a["physics_step"]: a for a in feedback["ordinary_flight"]["actions"]}
    updates = {u["physics_step"]: u for u in feedback["updates"] if u["phase"] == "maintained_terminal"}
    frames = feedback.get("controller_updates", [])
    if [f["physics_step"] for f in frames] != sorted(updates):
        raise ValueError("issued terminal frame inventory differs")
    for frame in frames:
        action = actions[frame["physics_step"]]
        if (frame["compute_time_us"] is not None or frame["frame"]["command"] != action["command"]
                or frame["sim_time_s"] != action["sim_time_s"]
                or frame["controller_update_index"] != action["controller_update_index"]
                or not frame["frame"]["metrics"]):
            raise ValueError("terminal frame command/clock/metrics binding differs")


def validate_mechanics(attempt, feedback):
    validate_finite_correction(attempt, feedback)
    candidate = feedback["candidate_id"]
    mode = next((m for m, value in MECHANICS_FAMILY.items() if value == candidate), None)
    metadata = attempt.get("planner_mechanics")
    if mode is None:
        if metadata is not None and metadata.get("enabled") is not False:
            raise ValueError("mechanics enabled under an older identity")
        return
    if metadata != {"enabled": True,
                    "waypoint_exit_consistency": mode in ("exit-consistency", "mechanics-combined", *FINITE_FAMILY),
                    "piecewise_early_target": mode in ("piecewise-early-target", "mechanics-combined"),
                    "recovery_lead": mode in ("recovery-lead", "mechanics-combined"),
                    "recovery_choices_recorded": True, "ordinary_default_changed": False,
                    "physical_guards_changed": False}:
        raise ValueError("mechanics ablation contract differs")
    for refresh in feedback["refreshes"]:
        if refresh["decision"] == "early_destination_obstruction":
            if (not metadata["piecewise_early_target"] or not refresh["goal"]["destination"]
                    or not refresh.get("previous_goal") or refresh["previous_goal"]["destination"]
                    or not refresh.get("desired_arc")
                    or (refresh.get("predicted_conflict") or {}).get("cause") != "ideal_arc_reserve"):
                raise ValueError("unbound early piecewise obstruction")
        query = refresh.get("recovery_query")
        if query is None:
            if refresh["decision"] in ("terrain_recovery_started", "terrain_recovery_command", "terrain_recovery_exhausted"):
                raise ValueError("missing recovery choice evidence")
            continue
        if (query["origin"] != refresh["origin"] or not 1 <= len(query["commands"]) <= 4
                or refresh["decision"] not in ("terrain_recovery_started", "terrain_recovery_command", "terrain_recovery_exhausted")):
            raise ValueError("recovery query origin/count differs")
        seen, safe = [], []
        for command in query["commands"]:
            if command["command"] in seen or command["command"]["throttle_frac"] != 1.0:
                raise ValueError("changed recovery command family")
            seen.append(command["command"])
            ticks = command["prediction_ticks"]
            conflict = command["conflict"]
            if command["checked"]:
                if not 24 <= ticks <= 240:
                    raise ValueError("invalid recovery query clock")
                if conflict is None:
                    safe.append(command)
                elif not 1 <= conflict["state"]["physics_step"] - query["origin"]["physics_step"] <= ticks:
                    raise ValueError("recovery conflict outside query")
            elif ticks >= 24 or conflict is not None:
                raise ValueError("unchecked recovery query claims a result")
        selected = refresh.get("terrain_correction")
        if refresh["decision"] == "terrain_recovery_exhausted":
            if safe or selected is not None:
                raise ValueError("exhaustion hides a safe selected command")
        elif (len(safe) != 1 or safe[0] != query["commands"][-1]
              or selected is None or selected["selected_command"] != safe[0]["command"]
              or selected["prediction_ticks"] != safe[0]["prediction_ticks"]
              or selected["selected_choice"] != safe[0]["choice"]):
            raise ValueError("recovery evidence and selected command differ")


def attempt(root, row, plan):
    command = [str(root / "candidate/bin/pd-eval"), "ballistic-feedback-flight", "--scenario",
               str(root / row["scenario_path"]), "--correction-cap", str(plan["correction_cap"]),
               "--output-dir", str(root / "runs" / row["attempt_id"])]
    if plan.get("waypoint_experiment") is not None:
        command += ["--waypoint-experiment", plan["waypoint_experiment"]]
    started = time.monotonic()
    try:
        process = subprocess.run(command, capture_output=True, timeout=plan["case_wall_limit_s"])
        out, err, code = process.stdout, process.stderr, process.returncode
        status = "recorded" if code == 0 else "runner_error"
    except subprocess.TimeoutExpired as error:
        out, err, code, status = error.stdout or b"", error.stderr or b"", None, "runner_timeout"
    except OSError as error:
        out, err, code, status = b"", str(error).encode(), None, "runner_error"
    study.write_new(root / "logs" / (row["attempt_id"] + ".stdout"), out)
    study.write_new(root / "logs" / (row["attempt_id"] + ".stderr"), err)
    actual = dict(row, output_dir="runs/" + row["attempt_id"], exit_code=code,
                  status=status, failure=None, result=None, wall_s=time.monotonic() - started)
    if status == "recorded":
        try:
            actual["result"] = check_record(root, actual, plan)
        except (ValueError, KeyError, TypeError, OSError) as error:
            actual.update(status="evidence_error", failure=str(error))
    return actual


def summary(rows):
    primary = [r for r in rows if r["cohort"] == "random"]
    recorded = [r for r in primary if r["status"] == "recorded"]
    landings = [r for r in recorded if r["result"]["verified_landing"]]
    baseline_landings = sum(r["baseline_result"]["verified_landing"] for r in primary)
    result = {"primary_count": len(primary), "recorded_count": len(recorded),
            "verified_landings": len(landings), "baseline_landings": baseline_landings,
            "zero_h_landings": sum(r["result"]["handoffs"] == 0 for r in landings),
            "waypoint_landings": sum(r["result"]["handoffs"] > 0 for r in landings),
            "dispositions": dict(sorted(Counter(r["status"] for r in primary).items())),
            "physical_outcomes": dict(sorted(Counter(r["result"]["physical_outcome"] for r in recorded).items())),
            "planning_stops": dict(sorted(Counter(r["result"]["stop_group"] for r in recorded).items())),
            "initial_candidate_arcs": dict(sorted(Counter(r["result"]["initial_candidate_arc"] for r in recorded).items())),
            "handoffs": dict(sorted(Counter(str(r["result"]["handoffs"]) for r in recorded).items())),
            "repeats_recorded": sum(r["cohort"] == "repeat" and r["status"] == "recorded" for r in rows),
            "lost_landings": [r["case_id"] for r in recorded if r["baseline_result"]["verified_landing"]
                              and not r["result"]["verified_landing"]],
            "gained_landings": [r["case_id"] for r in landings if not r["baseline_result"]["verified_landing"]],
            "per_original_cohort": {key: {"count": sum(r["baseline_result"]["nominal_class"] == key for r in primary),
                                          "landings": sum(r["baseline_result"]["nominal_class"] == key for r in landings)}
                                    for key in ("clear", "blocked")},
            "per_recipe": {key: {"count": sum(r["geometry"]["recipe_id"] == key for r in primary),
                                 "landings": sum(r["geometry"]["recipe_id"] == key for r in landings)}
                           for key in sorted({r["geometry"]["recipe_id"] for r in primary})}}
    if primary and "previous_result" in primary[0]:
        result.update(previous_candidate_landings=sum(r["previous_result"]["verified_landing"] for r in primary),
                      lost_previous_candidate_landings=[r["case_id"] for r in recorded
                          if r["previous_result"]["verified_landing"] and not r["result"]["verified_landing"]],
                      gained_previous_candidate_landings=[r["case_id"] for r in landings
                          if not r["previous_result"]["verified_landing"]],
                      waypoint_replacements=sum(r["result"]["waypoint_replacements"] for r in recorded),
                      rejected_proposals=sum(r["result"]["rejected_proposals"] for r in recorded))
    return result


def verify(root, *, contract_reader=None, gate_reader=None):
    plan = (contract_reader or contract)(root / "plan.json")
    receipt = d.read(root / "receipt.json")["files"]
    if survey.inventory(root) != receipt:
        raise ValueError("capture receipt inventory differs")
    survey.check_inventory(root, receipt)
    if gate_reader is not None:
        gate_reader(plan, root)
        if study.digest((root / "experiment-plan.md").read_bytes()) != plan["experiment_plan_sha256"]:
            raise ValueError("conditional sweep protocol differs")
    if plan.get("candidate_id") == COAST_CANDIDATE:
        from coast_terminal_validation import require_sweep_gate
        require_sweep_gate(plan, root)
        if study.digest((root / "experiment-plan.md").read_bytes()) != plan["experiment_plan_sha256"]:
            raise ValueError("conditional sweep protocol differs")
    manifest, report = d.read(root / "manifest.json"), d.read(root / "ballistic-feedback-sweep.json")
    previous = (root / "baseline-receipt.json").read_bytes()
    if study.digest(previous) != plan["baseline_receipt_sha256"]:
        raise ValueError("portable baseline receipt differs")
    baseline_files = json.loads(previous)["files"]
    saved = (root / "baseline-results.json").read_bytes()
    if (study.digest(saved) != plan["baseline_results_sha256"]
            or baseline_files["early-exit-sweep.json"] != study.digest(saved)):
        raise ValueError("portable baseline results differ")
    expected_cases = [r for r in json.loads(saved)["rows"] if r["cohort"] == "random"]
    previous = previous_results(plan, root)
    if len(expected_cases) != 1000 or len(manifest["cases"]) != 1000:
        raise ValueError("wrong primary denominator")
    for case, previous_row in zip(manifest["cases"], expected_cases):
        data = d.safe_file(root, case["scenario_path"]).read_bytes()
        if (case["case_id"] != previous_row["case_id"] or case["seed"] != previous_row["seed"]
                or case["geometry"] != previous_row["geometry"] or case["baseline_result"] != previous_row["result"]
                or study.digest(data) != case["scenario_sha256"]
                or study.digest(data) != baseline_files[case["scenario_path"]]):
            raise ValueError("original paired input/result binding differs")
        if previous is not None:
            old = previous[int(case["case_id"].split("-")[-1])]
            if (case["previous_result"] != old["result"] or case["scenario_sha256"] != old["scenario_sha256"]
                    or case["seed"] != old["seed"] or case["geometry"] != old["geometry"]):
                raise ValueError("prior candidate comparison binding differs")
    seal = d.read(root / "candidate/source-seal.json")
    if (seal != manifest["source"] or seal["executable_sha256"] != plan["candidate_executable_sha256"]
            or study.digest((root / "candidate/bin/pd-eval").read_bytes()) != seal["executable_sha256"]
            or study.digest((root / "candidate/bin/batch-report").read_bytes()) != manifest["renderer_sha256"]
            or report["manifest_sha256"] != study.digest((root / "manifest.json").read_bytes())):
        raise ValueError("candidate or renderer provenance differs")
    if ((plan.get("candidate_id") == COAST_CANDIDATE or gate_reader is not None)
            and manifest["renderer_sha256"] != plan["candidate_renderer_sha256"]):
        raise ValueError("conditional sweep renderer differs from admitted candidate")
    for path, digest in seal["files"].items():
        if study.digest(d.safe_file(root / "candidate/source", path).read_bytes()) != digest:
            raise ValueError("source snapshot differs")
    expected = attempts(manifest["cases"], plan)
    if len(report["rows"]) != len(expected):
        raise ValueError("wrong attempt inventory")
    for row, frozen in zip(report["rows"], expected):
        if any(row[k] != v for k, v in frozen.items() if k != "status"):
            raise ValueError("attempt identity/order differs")
        if row["status"] == "recorded" and check_record(root, row, plan) != row["result"]:
            raise ValueError("saved projection differs")
        if report["stopped_reason"] is None and row["status"] != "recorded":
            raise ValueError("incomplete collection marked complete")
    expected_after = manifest["source"]
    if report.get("collector_verifier_repair") is not None:
        repair = report["collector_verifier_repair"]
        path = "studies/terrain_profiles/ballistic_feedback_sweep.py"
        if (repair["path"] != path or repair["original_sha256"] != manifest["source"]["files"][path]
                or study.digest((root / "collector-verifier-source.py").read_bytes()) != repair["repaired_sha256"]):
            raise ValueError("unbound verifier-only repair")
        expected_after = dict(manifest["source"], files=dict(manifest["source"]["files"]))
        expected_after["files"][path] = repair["repaired_sha256"]
    if (summary(report["rows"]) != report["summary"]
            or report["source_after"] != expected_after or report["protected_after"] != manifest["protected"]):
        raise ValueError("summary/source/protected-state differs")
    return report


def run(args, *, contract_reader=None, gate_reader=None):
    plan = (contract_reader or contract)(args.plan)
    coast = plan.get("candidate_id") == COAST_CANDIDATE
    conditional = coast or gate_reader is not None
    if gate_reader is not None:
        gate_reader(plan)
    if coast:
        from coast_terminal_validation import require_sweep_gate, require_identity
        from ridge_waypoint_panel import rust_digest
        require_sweep_gate(plan)
    cases, receipt, bound = baseline(plan)
    native = survey.REPO / plan.get("native_path", "target/release/pd-eval")
    renderer = survey.REPO / plan.get("renderer_path", "target/release/examples/ballistic_feedback_batch_report")
    source = source_state(native)
    if source["executable_sha256"] != plan["candidate_executable_sha256"]:
        raise ValueError("candidate executable differs; do not rebuild or tune for this collection")
    if gate_reader is not None:
        from ridge_waypoint_panel import rust_digest
        if (rust_digest() != plan["candidate_rust_source_sha256"]
                or study.digest(renderer.read_bytes()) != plan["candidate_renderer_sha256"]
                or source["files"]["pd-report/src/planning_cycles.js"] != plan["candidate_report_script_sha256"]):
            raise ValueError("conditional sweep source/renderer differs from admitted candidate")
        protocol = d.safe_file(survey.REPO, plan["experiment_plan"]).read_bytes()
        if study.digest(protocol) != plan["experiment_plan_sha256"]:
            raise ValueError("conditional sweep protocol changed")
    if coast:
        require_identity({"executable_sha256": source["executable_sha256"],
                          "rust_source_tree_sha256": rust_digest(),
                          "renderer_sha256": study.digest(renderer.read_bytes())})
        if source["files"]["pd-report/src/planning_cycles.js"] != plan["candidate_report_script_sha256"]:
            raise ValueError("conditional sweep planning report differs")
        protocol = d.safe_file(survey.REPO, plan["experiment_plan"]).read_bytes()
        if study.digest(protocol) != plan["experiment_plan_sha256"]:
            raise ValueError("conditional sweep protocol changed")
    protected = protected_state(plan)
    root = args.output.resolve()
    survey.reserve(root)
    study.write_new(root / "plan.json", args.plan.read_bytes())
    if conditional:
        study.write_new(root / "experiment-plan.md", protocol)
        gate_panel = survey.REPO / plan["gate_panel_capture"]
        for source_name, output_name in [("receipt.json", "gate-panel-receipt.json"),
                                         ("ballistic-feedback-sweep.json", "gate-panel-results.json"),
                                         ("manifest.json", "gate-panel-manifest.json")]:
            study.write_new(root / output_name, (gate_panel / source_name).read_bytes())
        if gate_reader is not None:
            focus = survey.REPO / plan["gate_focus_capture"]
            for name in ("receipt.json", "ballistic-feedback-sweep.json", "manifest.json"):
                output_name = "gate-focus-" + ("results.json" if name == "ballistic-feedback-sweep.json" else name)
                study.write_new(root / output_name, (focus / name).read_bytes())
    study.write_new(root / "baseline-receipt.json", receipt)
    study.write_new(root / "baseline-results.json", bound("early-exit-sweep.json"))
    if "previous_capture" in plan:
        previous = survey.REPO / plan["previous_capture"]
        study.write_new(root / "previous-receipt.json", (previous / "receipt.json").read_bytes())
        study.write_new(root / "previous-results.json", (previous / "ballistic-feedback-sweep.json").read_bytes())
    study.write_new(root / "candidate/bin/pd-eval", native.read_bytes())
    (root / "candidate/bin/pd-eval").chmod(0o755)
    study.write_new(root / "candidate/bin/batch-report", renderer.read_bytes())
    (root / "candidate/bin/batch-report").chmod(0o755)
    study.write_new(root / "candidate/source-seal.json", study.encoded(source))
    for path, digest in source["files"].items():
        data = d.safe_file(survey.REPO, path).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("source preparation drift")
        study.write_new(root / "candidate/source" / path, data)
    for case in cases:
        study.write_new(root / case["scenario_path"], bound(case["scenario_path"]))
    manifest = {"schema": "pd-lab.ballistic-feedback-sweep-manifest.v1", "cases": cases,
                "source": source, "protected": protected, "renderer_sha256": study.digest(renderer.read_bytes())}
    study.write_new(root / "manifest.json", study.encoded(manifest))
    rows = attempts(cases, plan)
    study.write_new(root / "ledger-initial.json", study.encoded(rows))
    started, stopped = time.monotonic(), None
    # Waves bound unattended work after any infrastructure/evidence failure.
    with ThreadPoolExecutor(max_workers=plan["workers"]) as pool:
        for offset in range(0, len(rows), 20):
            if time.monotonic() - started >= plan["campaign_wall_limit_s"]:
                stopped = "campaign_wall_limit"
                break
            wave = list(pool.map(lambda row: attempt(root, row, plan), rows[offset:offset + 20]))
            rows[offset:offset + len(wave)] = wave
            study.write_new(root / "progress" / f"wave-{offset:04}.json", study.encoded(wave))
            stats = summary(rows)
            print(json.dumps({"recorded": stats["recorded_count"], "landings": stats["verified_landings"],
                              "stops": stats["planning_stops"], "wall_s": round(time.monotonic() - started, 1)}), flush=True)
            broken = next((r for r in wave if r["status"] != "recorded"), None)
            if broken:
                stopped = f'{broken["status"]}:{broken["attempt_id"]}:{broken["failure"]}'
                break
            if conditional:
                abnormal = next((r for r in wave if r["result"]["physical_outcome"]
                                 not in ("flying", "landed_on_target")), None)
                if abnormal:
                    stopped = f'abnormal_physical_outcome:{abnormal["attempt_id"]}:{abnormal["result"]["physical_outcome"]}'
                    break
            if source_state(native) != source or protected_state(plan) != protected:
                stopped = "source_or_protected_state_drift"
                break
    report = {"schema": "pd-lab.ballistic-feedback-sweep.v1", "rows": rows, "summary": summary(rows),
              "stopped_reason": stopped, "collection_wall_s": time.monotonic() - started,
              "source_after": source_state(native), "protected_after": protected_state(plan),
              "manifest_sha256": study.digest((root / "manifest.json").read_bytes())}
    study.write_new(root / "ballistic-feedback-sweep.json", study.encoded(report))
    rendered = subprocess.run([str(root / "candidate/bin/batch-report"), str(root)],
                              capture_output=True, timeout=120)
    study.write_new(root / "logs/renderer.stdout", rendered.stdout)
    study.write_new(root / "logs/renderer.stderr", rendered.stderr)
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    if rendered.returncode:
        raise ValueError("batch renderer failed; sealed native evidence retained")
    verify(root, contract_reader=contract_reader, gate_reader=gate_reader)
    print(json.dumps({"output": str(root), "stopped_reason": stopped, "summary": report["summary"]}), flush=True)
    if stopped:
        raise ValueError("diagnostic sweep stopped; sealed partial inventory retained")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    collect = commands.add_parser("run")
    collect.add_argument("--output", type=Path, required=True)
    collect.add_argument("--plan", type=Path, default=PLAN)
    check = commands.add_parser("verify")
    check.add_argument("capture", type=Path)
    args = parser.parse_args()
    if args.command == "run":
        run(args)
    else:
        result = verify(args.capture.resolve())
        print(json.dumps({"stopped_reason": result["stopped_reason"], "summary": result["summary"]}))


if __name__ == "__main__":
    main()
