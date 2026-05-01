* Pi-Network Attenuator
* Larger linear circuit with multiple shunt-and-series resistors.
* The 11-circuit set didn't include any 5+ device passive networks;
* this exercises the placer's multi-block handling on a clearly-
* directional but non-amplifier signal chain.
V1 in 0 AC=1
* First pi section
R1 in n1 50
R2 n1 0 100
R3 n1 n2 50
* Second pi section
R4 n2 0 100
R5 n2 n3 50
* Third pi section
R6 n3 0 100
R7 n3 out 50
R8 out 0 50
