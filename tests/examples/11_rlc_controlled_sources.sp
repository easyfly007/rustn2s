* RLC and Controlled Source Test
V1 in 0 AC=1
* Series RLC
R1 in n1 100
L1 n1 n2 10m
C1 n2 0 1u
* VCVS: voltage-controlled voltage source (gain=2)
E1 buf_out 0 n2 0 2
* VCCS: voltage-controlled current source (gm=1m)
G1 vdd gm_out n2 0 1m
R2 vdd gm_out 1k
* CCVS: current-controlled voltage source (transresistance=500)
V2 sense_in sense_out 0
H1 hout 0 V2 500
R3 sense_in 0 100
* CCCS: current-controlled current source (gain=10)
V3 fsense_in fsense_out 0
F1 vdd fout V3 10
R4 fsense_in 0 100
R5 vdd fout 1k
V4 vdd 0 5
