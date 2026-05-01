* Three Isolated Voltage Sources
* Stress for Phase 2.4 (fix_isolated_source_layers): three V sources
* whose only common net is ground. Circuit 14 already showed two
* sources collapse onto each other; this checks whether the same bug
* is N-way (three sources stacked on top of each other in one place).
V1 a 0 1
V2 b 0 2
V3 c 0 3
* Each source drives an independent load. No nets are shared between
* the three sub-circuits except ground.
R1 a 0 1k
R2 b 0 2k
R3 c 0 3k
