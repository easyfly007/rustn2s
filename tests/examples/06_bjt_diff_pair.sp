* BJT Differential Pair
V1 vcc 0 5
V2 inp 0 AC=0.5
V3 inm 0 AC=-0.5
I1 tail 0 1m
Q1 out1 inp tail NPN_MOD
Q2 out2 inm tail NPN_MOD
R1 vcc out1 5k
R2 vcc out2 5k
