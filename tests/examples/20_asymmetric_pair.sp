* Asymmetric "Diff Pair" with Mismatched W/L
* Two transistors that look like a diff pair (same source net) but
* have intentionally different W/L. The eval/symmetry module groups
* components by (symbol, W, L, model); since W differs, M1 and M2
* will NOT be matched as a pair. This is the FIRST test circuit where
* a visually-paired-looking layout ends up not contributing to the
* symmetry sub-score, which until now has saturated at 1.0 on every
* circuit.
V1 vcc 0 5
V2 inp 0 AC=0.5
V3 inm 0 AC=-0.5
I1 tail 0 1m
* Mismatched widths — looks like a pair, isn't one in eval terms.
M1 out1 inp tail 0 nch W=20u L=1u
M2 out2 inm tail 0 nch W=10u L=1u
R1 vcc out1 5k
R2 vcc out2 5k
