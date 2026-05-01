* PMOS-First Inverter (NMOS named M1, PMOS named M2)
* The Inverter pattern matcher in analyzer/mod.rs always emits
* (PMOS_first, NMOS_second) regardless of the netlist's device order,
* which is why circuit 09 (inverter_chain_hier) and others were
* always PMOS-above-NMOS. This circuit reverses the device-list
* order to test whether anything in the pipeline depends on netlist
* order vs. analyzer-imposed order.
V1 vdd 0 1.8
V2 vin 0 AC=0.5
* NMOS first in netlist, PMOS second
M1 vout vin 0 0 nch W=2u L=0.18u
M2 vout vin vdd vdd pch W=4u L=0.18u
C1 vout 0 50f
