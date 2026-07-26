# Generated bake-off fixtures

`scale_8x77.json` is a clean-room deterministic scale fixture for the expected
near-term workload. It contains eight heterogeneous containers and 77 uniquely
named items formed from seven repetitions of eleven dimension profiles.

Exact input-unit totals:

- item volume: 724,566.920;
- container capacity: 882,287.290; and
- theoretical utilization: 82.12% when all items fit.

The fixture is not derived from the old solver or its placement/scoring code.
Its checked-in JSON shape makes backend quality and deadline comparisons
repeatable across platforms.
