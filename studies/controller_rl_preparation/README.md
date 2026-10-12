# Shared controller/RL preparation artifacts

The [execution declaration](../../docs/controller_rl_preparation_execution.md)
owns this bounded allocation. These requests are committed before generation;
native `pd-missions` remains the generator and verification authority.

`prepare.py` inventories retained collection manifests, excluded proposals,
append-only usage receipts and all proposal records in training worker logs.
Its JSON inventory is evidence of the declared checkout history, not a claim
about unrecorded external runs. The compact exposure summary binds that inventory.
Older authored/ballistic terrain namespaces are separately reserved in the
summary; their unretained physical identities remain a stated audit limit.

The six initial requests allocate 5,376 proposal slots: two 128-slot development
stress support candidates, two 512-slot broad validation cohorts, and two
2,048-slot final-test cohorts. After the geometry-only pilot selects A or B, a
separate committed 512-slot stress request completes the 5,888-slot allocation.
The optional 128 additional resolution proposals remain unused initially.

The final-test collections receive only native structural/hash verification and
physical-identity reconciliation. Do not run `audit-v2`, render individual inputs
or supply them to ordinary branch workflows. Both terrain backends are known;
these cohorts can support an audited unseen-input claim, not an unseen-backend
claim.

Stress panels overlap. Support selection counts unique qualifying proposals
matching each panel before quota capacity; the minimum count wins, then total
memberships, then A. Sparse quotas remain explicit limits. Every original
proposal slot retains its disposition, with no accepted-count refill.

Run the preparation accounting tests with:

```sh
rtk proxy python3 -m unittest discover -s studies/controller_rl_preparation -p 'test_*.py'
```

Generation and native verification commands and their receipts are retained in
the new output namespace. These scripts perform no flight or training.
