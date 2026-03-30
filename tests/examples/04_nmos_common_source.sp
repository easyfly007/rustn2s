* NMOS Common-Source Amplifier
V1 vdd 0 3.3
V2 vin 0 AC=1
R1 vdd vdrain 5k
R2 vdd vgate 100k
R3 vgate 0 50k
R4 vsource 0 1k
C1 vin vgate 1u
C2 vdrain vout 1u
C3 vsource 0 10u
M1 vdrain vgate vsource 0 nch W=10u L=1u
