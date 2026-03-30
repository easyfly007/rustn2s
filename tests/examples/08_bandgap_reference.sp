* Bandgap Voltage Reference
V1 vdd 0 3.3
* PMOS current mirror
M1 branch1 bias vdd vdd pch W=20u L=2u
M2 branch2 bias vdd vdd pch W=20u L=2u
M3 bias bias vdd vdd pch W=20u L=2u
* BJT pair
Q1 0 branch1 e1 NPN_MOD
Q2 0 branch2 e2 NPN_MOD
* Resistors
R1 e1 0 10k
R2 e2 0 1k
R3 branch1 e1 5k
* Op-amp feedback forces branch voltages equal
E1 bias 0 branch1 branch2 1000
* Output
R4 vdd vref 20k
M4 vref bias vdd vdd pch W=20u L=2u
* Bypass cap
C1 vref 0 10p
