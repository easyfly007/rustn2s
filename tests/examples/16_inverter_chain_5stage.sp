* Five-Stage Inverter Chain (flat, not via subcircuit)
* Stresses the Sugiyama layered placer on a long linear DAG.
* Each stage is a CMOS inverter; the chain forces 5 distinct DAG
* layers, so the placer's longest-path layering should produce a
* clean left-to-right cascade.
V1 vdd 0 1.8
V2 vin 0 AC=0.5
* Stage 1
M1p s1 vin vdd vdd pch W=4u L=0.18u
M1n s1 vin 0 0 nch W=2u L=0.18u
* Stage 2
M2p s2 s1 vdd vdd pch W=4u L=0.18u
M2n s2 s1 0 0 nch W=2u L=0.18u
* Stage 3
M3p s3 s2 vdd vdd pch W=4u L=0.18u
M3n s3 s2 0 0 nch W=2u L=0.18u
* Stage 4
M4p s4 s3 vdd vdd pch W=4u L=0.18u
M4n s4 s3 0 0 nch W=2u L=0.18u
* Stage 5
M5p vout s4 vdd vdd pch W=4u L=0.18u
M5n vout s4 0 0 nch W=2u L=0.18u
C1 vout 0 50f
