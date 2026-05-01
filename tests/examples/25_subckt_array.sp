* Array of Subcircuit Instances (X devices)
* Three identical INV instances at top level, plus passives.
* Tests:
*   - Hierarchical mode rendering (X as boxes) when --hierarchical is on
*   - Flat mode rendering (X expanded to internal devices) by default
*   - Multi-instance handling: do three instances of the same subckt
*     end up co-aligned per their (symbol, W, L) match key?
* The 11-circuit set's 09_inverter_chain_hier had 3 X instances but in
* a chain; this one is parallel (independent loads), exercising a
* different DAG shape.
.subckt INV in out vdd vss
M1 out in vdd vdd pch W=4u L=0.18u
M2 out in vss vss nch W=2u L=0.18u
.ends INV

V1 vdd 0 1.8
V2 vin 0 AC=0.5
* Three parallel inverters with their own load caps
X1 vin out1 vdd 0 INV
X2 vin out2 vdd 0 INV
X3 vin out3 vdd 0 INV
C1 out1 0 100f
C2 out2 0 100f
C3 out3 0 100f
