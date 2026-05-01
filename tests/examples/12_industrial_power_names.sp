* Industrial Power-Net Names
* Stress-test for the hardcoded power_nets list in analyzer/mod.rs.
* Real industrial netlists use names like VBAT, VIO, VCORE that are
* not in the audit's identified hardcoded set {0/gnd/vss/vdd/vcc/avdd/avss}.
* Expected current behavior: VBAT/VIO are treated as ordinary signal nets,
* not power. This will likely show up as labels-in-the-air rather than
* power symbols, and may produce a high label_ratio score.
V1 vbat 0 3.3
V2 vio 0 1.8
* High-side load on VBAT
R1 vbat vmid 10k
C1 vmid 0 1u
* Low-side load on VIO
R2 vio vlow 5k
C2 vlow 0 10n
* Cross-rail divider
R3 vbat vlow 100k
