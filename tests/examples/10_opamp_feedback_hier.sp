* Op-Amp Feedback Circuit Using Subcircuit
.subckt OPAMP inp inm vdd vss out
M1 d1 inp tail vdd pch W=20u L=1u
M2 d2 inm tail vdd pch W=20u L=1u
M3 tail biasp vdd vdd pch W=40u L=2u
M4 d1 d1 vss vss nch W=10u L=1u
M5 d2 d1 vss vss nch W=10u L=1u
M6 out d2 vss vss nch W=40u L=1u
M7 out biasp vdd vdd pch W=80u L=2u
C1 d2 out 2p
I1 biasp vss 50u
M8 biasp biasp vdd vdd pch W=20u L=2u
.ends OPAMP

* Top level: non-inverting amplifier
V1 vdd 0 3.3
V2 vsig 0 AC=1
X1 vsig fb vdd 0 vout OPAMP
R1 vout fb 10k
R2 fb 0 10k
C1 vout 0 5p
