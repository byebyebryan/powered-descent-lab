# Existing-input V2 incumbent baseline

This declaration measures the unchanged cap-24 acquisition/terminal-safety
reference on the already exposed V2 calibration inputs. See the
[run contract](../../../docs/procedural_challenge_baseline.md).

`declaration.json` and the eight exact campaign request files are created before
flight and committed with the integration source. The main schedules contain
every admitted index in the first 2,048 original proposal slots per backend;
the integration schedules use the first eight admitted indices in each main
schedule. Integration repeats are kept separate from headline rates.

Run the already-built frozen binaries with:

```sh
rtk proxy python3 -B studies/mission_coverage/run_incumbent_v2.py --output outputs/procedural-challenge/NEW-BASELINE
```

The runner creates new local campaign/capture/proof/report artifacts and appends
evaluation exposure receipts. It never rebuilds, regenerates inputs, tunes a
controller, trains a policy, changes accepted navigation or starts/stops a server.
