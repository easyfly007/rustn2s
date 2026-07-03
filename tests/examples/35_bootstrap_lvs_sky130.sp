* Bootstrap Sampling Switch, LVS Schematic (SKY130) [n2s test case 35]
*
* === n2s test-case metadata ===
* Source:       myadc repo, sky130/layout/bootstrap/bootstrap_schematic.spice
* Circuit type: Modified Abo-Gray bootstrapped sampling switch, flat
* Devices:      13 total -- 12 transistors + 1 MIM cap, all SKY130 X instances
* MOSFETs:      12 (7 nfet, 5 pfet)
* Notes:        Bulk terminals sit on NON-power signal nets (triple-well
*               body=out on XM1/XM5, isolated-nwell body=nt on XM3/XM6/
*               XM8/XM10). NOTE: these are X instances, so M-card MOS type
*               inference (audit item C2) is NOT exercised -- they render
*               as subckt boxes. What this case does test: body pins wired
*               to signal nets, and an X-instance MIM cap (XCboot).
*               True C2 (an M card with non-matching model name AND signal
*               bulk) remains uncovered; no real myadc netlist has one.
* Added:        2026-07-03  (real-world test-set enrichment, batch 2)
* ===============================
* Bootstrap Sampling Switch - Schematic for LVS
* Extracted from sky130/bootstrap/bootstrap_sw.spice (without testbench)
* Modified Abo-Gray topology: 10 MOSFETs + clock inverter + 2pF MIM cap
*
* Well domains:
*   Triple-well (body=out): M1, M5
*   Isolated nwell (body=nt): M3, M6, M8, M10
*   Standard psub (body=vss): M2, M4, M7, M9
*   Standard nwell (body=vdd): INV PMOS
*   Standard psub (body=vss): INV NMOS

.subckt bootstrap_sw in out phi vdd vss

*** Clock inverter ***
XPC phib phi vdd vdd sky130_fd_pr__pfet_01v8 W=1.0 L=0.15
XNC phib phi vss vss sky130_fd_pr__nfet_01v8 W=0.42 L=0.15

*** Main sampling NMOS (triple-well, body=out) ***
XM1 in gate out out sky130_fd_pr__nfet_01v8 W=2.0 L=0.15

*** Bootstrap capacitor (MIM cap, ~2pF) ***
XCboot nt nb sky130_fd_pr__cap_mim_m3_1 W=31.7 L=31.7

*** HOLD phase transistors ***
XM2 nb phib vss vss sky130_fd_pr__nfet_01v8 W=1.0 L=0.15
XM3 nt m3g vdd nt sky130_fd_pr__pfet_01v8 W=1.0 L=0.15
XM9 m3g phib phi vss sky130_fd_pr__nfet_01v8 W=0.42 L=0.15
XM10 m3g phib nt nt sky130_fd_pr__pfet_01v8 W=1.0 L=0.15
XM4 gate phib vss vss sky130_fd_pr__nfet_01v8 W=1.0 L=0.15

*** SAMPLE phase transistors ***
XM7 m5g phib vss vss sky130_fd_pr__nfet_01v8 W=1.0 L=0.15
XM8 m5g phib nt nt sky130_fd_pr__pfet_01v8 W=1.0 L=0.15
XM5 nb m5g out out sky130_fd_pr__nfet_01v8 W=1.0 L=0.15
XM6 nt phib gate nt sky130_fd_pr__pfet_01v8 W=1.0 L=0.15

.ends bootstrap_sw
