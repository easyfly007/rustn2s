* Sampling Switch Array, 9 CMOS TGs (SKY130) [n2s test case 33]
*
* === n2s test-case metadata ===
* Source:       myadc repo, sky130/layout/sampling_sw/sampling_sw_schematic.spice
* Circuit type: Sampling switch array -- 1 reset TG + 8 sampling TGs
* Devices:      18 total -- 18 transistors as SKY130 X instances (9 nfet, 9 pfet)
* MOSFETs:      18, sky130_fd_pr__*fet_01v8 subckt instances, no local .subckt def
* Notes:        Highly repetitive structure (9 identical TG pairs) -- stresses
*               the pair-aware Unknown block template and label dedup on the
*               shared vin/phi/phi_b nets.
* Added:        2026-07-03  (real-world test-set enrichment, batch 2)
* ===============================
* Sampling Switch Array — Schematic for LVS
* 9 CMOS TGs: 1 reset (vcm→vtop) + 8 sampling (vin→bottom plates)
* Each TG: NMOS(W=1.0, L=0.15) + PMOS(W=2.0, L=0.15)

.subckt sampling_sw phi phi_b vin vcm vtop bm7 bm6 bm5 bm4 bl3 bl2 bl1 bl0 vdd vss

* Reset TG: vcm → vtop
XSN_rst vtop phi vcm vss sky130_fd_pr__nfet_01v8 W=1.0 L=0.15
XSP_rst vtop phi_b vcm vdd sky130_fd_pr__pfet_01v8 W=2.0 L=0.15

* Sampling TGs: vin → bottom plates
XSN_s7 bm7 phi vin vss sky130_fd_pr__nfet_01v8 W=1.0 L=0.15
XSP_s7 bm7 phi_b vin vdd sky130_fd_pr__pfet_01v8 W=2.0 L=0.15

XSN_s6 bm6 phi vin vss sky130_fd_pr__nfet_01v8 W=1.0 L=0.15
XSP_s6 bm6 phi_b vin vdd sky130_fd_pr__pfet_01v8 W=2.0 L=0.15

XSN_s5 bm5 phi vin vss sky130_fd_pr__nfet_01v8 W=1.0 L=0.15
XSP_s5 bm5 phi_b vin vdd sky130_fd_pr__pfet_01v8 W=2.0 L=0.15

XSN_s4 bm4 phi vin vss sky130_fd_pr__nfet_01v8 W=1.0 L=0.15
XSP_s4 bm4 phi_b vin vdd sky130_fd_pr__pfet_01v8 W=2.0 L=0.15

XSN_s3 bl3 phi vin vss sky130_fd_pr__nfet_01v8 W=1.0 L=0.15
XSP_s3 bl3 phi_b vin vdd sky130_fd_pr__pfet_01v8 W=2.0 L=0.15

XSN_s2 bl2 phi vin vss sky130_fd_pr__nfet_01v8 W=1.0 L=0.15
XSP_s2 bl2 phi_b vin vdd sky130_fd_pr__pfet_01v8 W=2.0 L=0.15

XSN_s1 bl1 phi vin vss sky130_fd_pr__nfet_01v8 W=1.0 L=0.15
XSP_s1 bl1 phi_b vin vdd sky130_fd_pr__pfet_01v8 W=2.0 L=0.15

XSN_s0 bl0 phi vin vss sky130_fd_pr__nfet_01v8 W=1.0 L=0.15
XSP_s0 bl0 phi_b vin vdd sky130_fd_pr__pfet_01v8 W=2.0 L=0.15

.ends sampling_sw
