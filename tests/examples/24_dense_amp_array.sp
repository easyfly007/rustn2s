* Dense 3-Stage Amplifier Array (~30 devices)
* Three independent NMOS common-source stages sharing only VDD/GND.
* This is the largest flat (no-subckt) circuit in the suite, designed
* to stress placer scaling — multi-column grid distribution, label vs
* wire balance, and the placer's HAC behavior on a moderate device
* count (close to the original max_cluster_size of 6 per cluster).
V1 vdd 0 3.3
V2 in1 0 AC=0.5
V3 in2 0 AC=0.5
V4 in3 0 AC=0.5
* Stage A
M1a vd1 in1 0 0 nch W=10u L=0.5u
R1a vdd vd1 5k
R2a vdd g1 100k
R3a g1 0 50k
R4a s1 0 1k
C1a in1 g1 1u
C2a vd1 outa 1u
C3a s1 0 10u
* Stage B
M1b vd2 in2 0 0 nch W=10u L=0.5u
R1b vdd vd2 5k
R2b vdd g2 100k
R3b g2 0 50k
R4b s2 0 1k
C1b in2 g2 1u
C2b vd2 outb 1u
C3b s2 0 10u
* Stage C
M1c vd3 in3 0 0 nch W=10u L=0.5u
R1c vdd vd3 5k
R2c vdd g3 100k
R3c g3 0 50k
R4c s3 0 1k
C1c in3 g3 1u
C2c vd3 outc 1u
C3c s3 0 10u
* Common load
R_load outa outb 10k
R_load2 outb outc 10k
