* Inverter Chain with Subcircuit
.subckt INV in out vdd vss
M1 out in vdd vdd pch W=20u L=1u
M2 out in vss vss nch W=10u L=1u
.ends INV

.subckt BUF in out vdd vss
X1 in mid vdd vss INV
X2 mid out vdd vss INV
.ends BUF

V1 vdd 0 3.3
X1 input net1 vdd 0 INV
X2 net1 net2 vdd 0 BUF
X3 net2 output vdd 0 INV
C1 output 0 1p
