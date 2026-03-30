* Two-Stage CMOS Op-Amp
V1 vdd 0 3.3
V2 inp 0 AC=0.5
V3 inm 0 AC=-0.5
* Input diff pair (PMOS)
M1 drain1 inp tail vdd pch W=20u L=1u
M2 drain2 inm tail vdd pch W=20u L=1u
* Tail current source
M3 tail bias_p vdd vdd pch W=40u L=2u
* NMOS active load (current mirror)
M4 drain1 drain1 0 0 nch W=10u L=1u
M5 drain2 drain1 0 0 nch W=10u L=1u
* Second stage: common-source
M6 vout drain2 0 0 nch W=40u L=1u
M7 vout bias_p vdd vdd pch W=80u L=2u
* Bias generation
M8 bias_p bias_p vdd vdd pch W=20u L=2u
I1 bias_p 0 50u
* Compensation
C1 drain2 vout 2p
R1 drain2 comp_mid 500
C2 comp_mid vout 1p
* Load
C3 vout 0 5p
