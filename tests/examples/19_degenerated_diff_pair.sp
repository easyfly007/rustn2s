* Source-Degenerated Differential Pair
* Real diff pairs often have a small resistor between each transistor's
* source and the tail node (degeneration). This breaks the analyzer's
* "Q1.source == Q2.source" invariant — neither transistor sees `tail`
* directly. The textbook diff_pair recognizer (audit C3) should fail
* here, and the analyzer will fall back to HAC clustering. Useful as a
* test that the pattern matcher's failure mode is graceful.
V1 vcc 0 5
V2 inp 0 AC=0.5
V3 inm 0 AC=-0.5
I1 tail 0 1m
* Degeneration resistors between transistor sources and the tail.
RE1 e1 tail 100
RE2 e2 tail 100
* NPN diff pair — sources are e1/e2, NOT the same node
Q1 out1 inp e1 NPN_MOD
Q2 out2 inm e2 NPN_MOD
* Collector loads
R1 vcc out1 5k
R2 vcc out2 5k
