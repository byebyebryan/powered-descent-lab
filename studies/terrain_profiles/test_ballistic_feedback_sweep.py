import copy
import unittest

import ballistic_feedback_sweep as sweep


def feedback():
    final = {"physical_outcome": "landed_on_target", "mission_outcome": "success",
             "physics_step": 5880, "sim_time_s": 49.0}
    return {"candidate_id": "ballistic_feedback_v1", "prefix_origin": None, "prefix_flight_sha256": None,
            "terrain_neutral_diagnostic": False, "integrity_passed": True, "source_replay_passed": True,
            "decisions_reproduced": True, "stop": "physical_terminal", "final_state": final,
            "ordinary_flight": {"final_state": copy.deepcopy(final)}, "refreshes": [], "handoffs": []}


def terrain_feedback():
    value = feedback()
    value.update(candidate_id=sweep.TERRAIN_CANDIDATE, handoff_goal_revisions=[])
    goal = {"number": 0, "revision": 0, "destination": True, "position_m": {"x": 100, "y": 5}}
    command = {"throttle_frac": 1.0, "target_attitude_rad": 0.0}
    proof = {"episode_start_physics_step": 100, "prediction_ticks": 24,
             "selected_choice": "upright_lift", "selected_command": command,
             "rejected_command": dict(command, throttle_frac=0.0)}
    value["refreshes"] = [
        {"decision": decision, "goal": copy.deepcopy(goal), "origin": {"physics_step": tick},
         "desired_arc": None, "predicted_command": command,
         "terrain_correction": copy.deepcopy(proof) if decision in ("terrain_recovery_started", "terrain_recovery_command") else None}
        for decision, tick in [("powered_correction", 0), ("terrain_recovery_started", 100),
                               ("terrain_recovery_command", 200), ("terrain_recovery_resumed", 202)]]
    value["updates"] = [{"physics_step": tick, "command": command, "phase": "terrain_clearance_recovery"}
                        for tick in (100, 200)]
    return value


class SweepTests(unittest.TestCase):
    def test_countdown_is_single_admission_and_never_promoted_under_old_identity(self):
        metadata = {"enabled": True, "rule": "retain_first_selected_ballistic_fallback_arrival",
                    "admissions_per_landing": 1, "remaining_time_floor_s": 0.5,
                    "revalidate_each_update": True, "release_on_infeasible_or_expired": True,
                    "target_convention_changed": False, "ordinary_default_changed": False}
        for candidate in sweep.COUNTDOWN_CANDIDATES.values():
            sweep.validate_landing_countdown({"landing_countdown": metadata}, candidate)
            with self.assertRaises(ValueError):
                sweep.validate_landing_countdown({}, candidate)
            for key, value in (("enabled", False), ("admissions_per_landing", 2),
                               ("revalidate_each_update", False), ("target_convention_changed", True),
                               ("release_on_infeasible_or_expired", False)):
                with self.assertRaises(ValueError):
                    sweep.validate_landing_countdown({"landing_countdown": dict(metadata, **{key: value})}, candidate)
        sweep.validate_landing_countdown({}, sweep.LANDING_CANDIDATES["landing-duration"])
        with self.assertRaises(ValueError):
            sweep.validate_landing_countdown({"landing_countdown": metadata}, sweep.LANDING_CANDIDATES["landing-duration"])

    def test_landing_duration_is_explicit_single_bounded_shared_query(self):
        metadata = {"enabled": True, "rule": "initial_vertical_balance_after_all_latest_safe_fits_fail",
                    "additional_queries": 1, "burn_time_min_s": 3.0, "burn_time_max_s": 14.0,
                    "shared_entry_and_live_controller": True, "ordinary_default_changed": False}
        for candidate in sweep.LANDING_FAMILY.values():
            sweep.validate_landing_duration({"landing_duration": metadata}, candidate)
            with self.assertRaises(ValueError):
                sweep.validate_landing_duration({}, candidate)
            for key, value in (("enabled", False), ("additional_queries", 2), ("burn_time_max_s", 30.0),
                               ("shared_entry_and_live_controller", False), ("ordinary_default_changed", True)):
                with self.assertRaises(ValueError):
                    sweep.validate_landing_duration({"landing_duration": dict(metadata, **{key: value})}, candidate)
        sweep.validate_landing_duration({}, sweep.LOCAL_CANDIDATES["early-target"])
        sweep.validate_landing_duration({"landing_duration": dict(metadata, enabled=False)}, sweep.RIDGE_CANDIDATE)
        with self.assertRaises(ValueError):
            sweep.validate_landing_duration({"landing_duration": metadata}, sweep.RIDGE_CANDIDATE)

    def test_local_modes_keep_complete_source_proof_requirements(self):
        for candidate in sweep.LOCAL_FAMILY.values():
            value = feedback()
            value.update(candidate_id=candidate, handoff_goal_revisions=[], updates=[])
            result = sweep.projection(value, candidate)
            self.assertEqual(result["early_destination_reacquisitions"], 0)
            value["decisions_reproduced"] = False
            with self.assertRaises(ValueError):
                sweep.projection(value, candidate)

    def test_early_destination_change_is_not_counted_as_a_handoff(self):
        value = feedback()
        candidate = sweep.LOCAL_CANDIDATES["early-target"]
        value.update(candidate_id=candidate, handoff_goal_revisions=[], updates=[])
        target = {"x": 100.0, "y": 5.0}
        value["refreshes"] = [{"decision": "destination_reacquired_before_waypoint",
            "origin": {"physics_step": 24, "held_command": {"throttle_frac": 0.0}},
            "goal": {"number": 0, "revision": 2, "destination": True, "position_m": target},
            "previous_goal": {"number": 1, "revision": 1, "destination": False},
            "desired_arc": {"target_m": target}, "terrain_conflict_m": None,
            "waypoint_braking_room": {"required_distance_m": 120.0, "remaining_room_m": -90.0}}]
        result = sweep.projection(value, candidate)
        self.assertEqual(result["early_destination_reacquisitions"], 1)
        self.assertEqual(result["handoffs"], 0)
        for mutate in (lambda r: r["waypoint_braking_room"].update(remaining_room_m=1.0),
                       lambda r: r["previous_goal"].update(destination=True),
                       lambda r: r["origin"]["held_command"].update(throttle_frac=1.0)):
            bad = copy.deepcopy(value)
            mutate(bad["refreshes"][0])
            with self.assertRaises(ValueError):
                sweep.projection(bad, candidate)

    def test_entry_ablation_identities_still_require_complete_native_proofs(self):
        for mode, candidate in sweep.ENTRY_CANDIDATES.items():
            value = feedback()
            value.update(candidate_id=candidate, handoff_goal_revisions=[], updates=[])
            self.assertEqual(sweep.projection(value, candidate)["waypoint_acquisition_continuations"], 0)
            value["source_replay_passed"] = False
            with self.assertRaises(ValueError, msg=mode):
                sweep.projection(value, candidate)

    def test_ridge_selection_binds_crest_and_body_offset(self):
        value = feedback()
        value.update(candidate_id=sweep.RIDGE_CANDIDATE, handoff_goal_revisions=[], updates=[])
        selection = {"kind": "blocking_ridge", "conflict_x_m": 60.0, "scan_start_x_m": 53.6,
                     "scan_end_x_m": 180.0, "crest_m": {"x": 100.0, "y": 100.0},
                     "separating_drop_m": 100.0, "diameter_m": 12.8}
        value["refreshes"] = [{"decision": "waypoint_selected", "desired_arc": None,
            "origin": {"physics_step": 0, "position_m": {"x": 0.0, "y": 20.0}},
            "goal": {"number": 1, "revision": 1, "destination": False, "position_m": {"x": 112.8, "y": 140.0}},
            "replan_trigger": {"cause": "ideal_arc_reserve", "state": {"position_m": {"x": 60.0, "y": 50.0}}},
            "ridge_selection": selection}]
        self.assertEqual(sweep.projection(value, sweep.RIDGE_CANDIDATE)["ridge_waypoint_proposals"], 1)
        for mutate in (
                lambda r: r["goal"]["position_m"].update(x=101.0),
                lambda r: r["ridge_selection"].update(separating_drop_m=6.0),
                lambda r: r.update(ridge_selection=None)):
            bad = copy.deepcopy(value)
            mutate(bad["refreshes"][0])
            with self.assertRaises(ValueError):
                sweep.projection(bad, sweep.RIDGE_CANDIDATE)

    def test_local_climb_stage_is_not_a_cleared_ridge(self):
        value = feedback()
        value.update(candidate_id=sweep.RIDGE_CANDIDATE, handoff_goal_revisions=[], updates=[])
        value["refreshes"] = [{"decision": "waypoint_selected", "desired_arc": None,
            "origin": {"physics_step": 0, "position_m": {"x": 0.0, "y": 20.0}},
            "goal": {"number": 1, "revision": 1, "destination": False, "position_m": {"x": 72.8, "y": 140.0}},
            "replan_trigger": {"cause": "ideal_arc_reserve", "state": {"position_m": {"x": 60.0, "y": 50.0}}},
            "ridge_selection": {"kind": "local_climb_stage", "conflict_x_m": 60.0,
                "scan_start_x_m": 53.6, "scan_end_x_m": 787.2, "crest_m": None,
                "separating_drop_m": 0.0, "diameter_m": 12.8}}]
        result = sweep.projection(value, sweep.RIDGE_CANDIDATE)
        self.assertEqual(result["ridge_waypoint_proposals"], 0)
        self.assertEqual(result["local_climb_stage_proposals"], 1)

    def test_requires_complete_landing_proof(self):
        self.assertTrue(sweep.projection(feedback())["verified_landing"])
        for key in ("integrity_passed", "source_replay_passed", "decisions_reproduced"):
            value = feedback()
            value[key] = False
            with self.assertRaises(ValueError):
                sweep.projection(value)

    def test_rejects_neutral_or_restored_prefix(self):
        for key, bad in (("terrain_neutral_diagnostic", True), ("prefix_origin", {}), ("prefix_flight_sha256", "x")):
            value = feedback()
            value[key] = bad
            with self.assertRaises(ValueError):
                sweep.projection(value)

    def test_finite_stop_not_crash_or_landing(self):
        value = feedback()
        value["stop"] = "short_command_rejected:body_reserve"
        value["final_state"].update(physical_outcome="flying", mission_outcome="in_progress")
        value["ordinary_flight"]["final_state"] = copy.deepcopy(value["final_state"])
        actual = sweep.projection(value)
        self.assertFalse(actual["verified_landing"])
        self.assertEqual(actual["physical_outcome"], "flying")
        self.assertEqual(actual["stop_group"], "short_command_rejected")

    def test_repeats_do_not_inflate_denominator(self):
        result = sweep.projection(feedback())
        row = {"cohort": "random", "status": "recorded", "case_id": "random-000", "result": result,
               "geometry": {"recipe_id": "test"}, "baseline_result": {"verified_landing": True, "nominal_class": "clear"}}
        stats = sweep.summary([row, dict(row, cohort="repeat")])
        self.assertEqual(stats["primary_count"], 1)
        self.assertEqual(stats["verified_landings"], 1)
        self.assertEqual(stats["repeats_recorded"], 1)

    def test_incomplete_physical_mission_tuple_not_counted(self):
        value = feedback()
        value["final_state"]["mission_outcome"] = "failure"
        value["ordinary_flight"]["final_state"] = copy.deepcopy(value["final_state"])
        self.assertFalse(sweep.projection(value)["verified_landing"])

    def test_land_and_aim_stop_mismatch_rejected(self):
        value = feedback()
        value["stop"] = "no_ballistic_aim"
        with self.assertRaises(ValueError):
            sweep.projection(value)

    def test_replan_identity_and_handoffs_bind_the_accepted_revision(self):
        value = feedback()
        value["candidate_id"] = "ballistic_feedback_v2_replan"
        value["handoffs"] = [{"physics_step": 10}]
        value["handoff_goal_revisions"] = [2]
        value["refreshes"] = [{"decision": "waypoint_replaced", "desired_arc": None,
                               "goal": {"number": 1, "revision": 2, "destination": False},
                               "origin": {"physics_step": 0}}]
        self.assertEqual(sweep.projection(value, value["candidate_id"])["waypoint_replacements"], 1)
        with self.assertRaises(ValueError):
            sweep.projection(value)  # v1 remains explicitly pinned.
        value["handoff_goal_revisions"] = [1]
        with self.assertRaises(ValueError):
            sweep.projection(value, value["candidate_id"])
        value["handoff_goal_revisions"] = [2]
        value["refreshes"][0]["decision"] = "waypoint_proposal_blocked"
        with self.assertRaises(ValueError):
            sweep.projection(value, value["candidate_id"])

    def test_previous_candidate_comparison_is_separate_from_old_baseline(self):
        result = sweep.projection(feedback())
        result.update(waypoint_replacements=1, rejected_proposals=0)
        row = {"cohort": "random", "status": "recorded", "case_id": "random-000", "result": result,
               "geometry": {"recipe_id": "test"},
               "baseline_result": {"verified_landing": True, "nominal_class": "clear"},
               "previous_result": {"verified_landing": False}}
        stats = sweep.summary([row, dict(row, cohort="repeat")])
        self.assertEqual(stats["baseline_landings"], 1)
        self.assertEqual(stats["previous_candidate_landings"], 0)
        self.assertEqual(stats["gained_previous_candidate_landings"], ["random-000"])
        self.assertEqual(stats["waypoint_replacements"], 1)

    def test_same_goal_reacquisition_does_not_duplicate_accepted_goal_identity(self):
        value = feedback()
        value["candidate_id"] = "ballistic_feedback_v2_replan_r2"
        value["handoffs"] = [{"physics_step": 10}]
        value["handoff_goal_revisions"] = [1]
        selected = {"decision": "waypoint_selected", "desired_arc": None,
                    "goal": {"number": 1, "revision": 1, "destination": False},
                    "origin": {"physics_step": 0}}
        value["refreshes"] = [selected, dict(selected, decision="waypoint_reacquired")]
        result = sweep.projection(value, value["candidate_id"])
        self.assertEqual(result["waypoint_selections"], 1)
        self.assertEqual(result["waypoint_replacements"], 0)
        self.assertEqual(result["waypoint_reacquisitions"], 1)

    def test_recovery_keeps_goal_and_binds_the_actual_commands(self):
        value = terrain_feedback()
        result = sweep.projection(value, sweep.TERRAIN_CANDIDATE)
        self.assertEqual(result["terrain_recovery_episodes"], 1)
        self.assertEqual(result["terrain_recovery_resumed"], 1)
        self.assertEqual(result["terrain_recovery_updates"], 2)
        bad = copy.deepcopy(value)
        bad["refreshes"][2]["goal"]["revision"] = 1
        with self.assertRaises(ValueError):
            sweep.projection(bad, sweep.TERRAIN_CANDIDATE)
        bad = copy.deepcopy(value)
        bad["updates"][0]["command"]["throttle_frac"] = 0.0
        with self.assertRaises(ValueError):
            sweep.projection(bad, sweep.TERRAIN_CANDIDATE)

    def test_short_warning_cannot_be_relabelled_as_waypoint_replacement(self):
        value = terrain_feedback()
        value["refreshes"][0].update(decision="waypoint_selected", replan_trigger={"cause": "short_command_reserve"})
        with self.assertRaises(ValueError):
            sweep.projection(value, sweep.TERRAIN_CANDIDATE)

    def test_recovery_bounds_and_unbound_commands_are_rejected(self):
        value = terrain_feedback()
        value["refreshes"][2]["terrain_correction"]["prediction_ticks"] = 241
        with self.assertRaises(ValueError):
            sweep.projection(value, sweep.TERRAIN_CANDIDATE)

    def test_waypoint_height_repair_binds_same_state_and_changes_only_height(self):
        goal = {"number": 1, "revision": 1, "destination": False, "position_m": {"x": 100, "y": 50}}
        origin = {"physics_step": 100, "sim_time_s": 100 / 120}
        previous = {"decision": "waypoint_proposal_blocked", "origin": origin, "goal": goal,
                    "desired_arc": {"steps": 200}}
        repair = {"from_goal": copy.deepcopy(goal), "height_increase_m": 10.0,
                  "handoff_height_tolerance_m": 6.4, "checked_through_relative_tick": 440,
                  "limiting_conflict": {"state": {"physics_step": 350},
                                        "cause": "waypoint_ideal_continuation_reserve"}}
        current = {"decision": "waypoint_selected", "origin": copy.deepcopy(origin),
                   "goal": dict(goal, revision=2, position_m={"x": 100, "y": 60}),
                   "waypoint_height_repair": repair}
        value = {"refreshes": [previous, current]}
        sweep.validate_waypoint_repairs(value)
        for field in ("x", "y"):
            bad = copy.deepcopy(value)
            bad["refreshes"][1]["goal"]["position_m"][field] += 1
            with self.assertRaises(ValueError):
                sweep.validate_waypoint_repairs(bad)
        bad = copy.deepcopy(value)
        bad["refreshes"][1]["origin"]["physics_step"] += 2
        with self.assertRaises(ValueError):
            sweep.validate_waypoint_repairs(bad)

        for lift in (float("nan"), float("inf"), 0, -1):
            bad = copy.deepcopy(value)
            bad["refreshes"][1]["waypoint_height_repair"]["height_increase_m"] = lift
            with self.assertRaises(ValueError):
                sweep.validate_waypoint_repairs(bad)

    def test_waypoint_height_repair_rejects_unbound_metadata(self):
        value = {"refreshes": [{"waypoint_height_repair": {"height_increase_m": 10}}]}
        with self.assertRaises(ValueError):
            sweep.validate_waypoint_repairs(value)
        value = terrain_feedback()
        value["updates"].append(dict(value["updates"][0], physics_step=300))
        with self.assertRaises(ValueError):
            sweep.projection(value, sweep.TERRAIN_CANDIDATE)


if __name__ == "__main__":
    unittest.main()
