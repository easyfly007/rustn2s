* TG-Based Master-Slave D Flip-Flop (PTM 28nm) [n2s test case 31]
*
* === n2s test-case metadata ===
* Source:       myadc repo, ptm28nm/layout/tech/schematics/dff.spice
* Circuit type: Transmission-gate master-slave DFF, LVS schematic reference
* Devices:      16 total -- 16 MOSFET (8 nfet, 8 pfet), subckt-only netlist
* MOSFETs:      16; model names are bare `nfet`/`pfet`, which do NOT match
*               the nch/nmos/pch/pmos keyword check -- type inference must
*               fall back to the bulk-node rule (VSS -> NMOS, VDD -> PMOS).
* Notes:        Master/slave feedback loops make the connectivity graph
*               cyclic (non-DAG); TG pass gates have signal-only D/S paths.
* Added:        2026-07-03  (real-world test-set enrichment, batch 2)
* ===============================
* DFF schematic reference for LVS (flat, 20 MOSFETs)
* TG-based master-slave D flip-flop
* Standard INV: NMOS W=250n, PMOS W=500n
* Weak INV: NMOS W=100n, PMOS W=200n
* TG: NMOS W=250n, PMOS W=500n
.subckt DFF D CLK Q QB VDD VSS
* CLK inverter: CLK -> clkb
MN1 clkb CLK VSS VSS nfet w=250n l=28n
MP1 clkb CLK VDD VDD pfet w=500n l=28n
* TG1: D -> m1 (pass when clk=0, NMOS gate=clkb, PMOS gate=CLK)
MN2 D clkb m1 VSS nfet w=250n l=28n
MP2 D CLK m1 VDD pfet w=500n l=28n
* Master inverter: m1 -> m1b
MN3 m1b m1 VSS VSS nfet w=250n l=28n
MP3 m1b m1 VDD VDD pfet w=500n l=28n
* Master feedback (weak): m1b -> m1
MN4 m1 m1b VSS VSS nfet w=100n l=28n
MP4 m1 m1b VDD VDD pfet w=200n l=28n
* TG2: m1b -> m2 (pass when clk=1, NMOS gate=CLK, PMOS gate=clkb)
MN5 m1b CLK m2 VSS nfet w=250n l=28n
MP5 m1b clkb m2 VDD pfet w=500n l=28n
* Slave output inverter: m2 -> Q
MN6 Q m2 VSS VSS nfet w=250n l=28n
MP6 Q m2 VDD VDD pfet w=500n l=28n
* Complement inverter: Q -> QB
MN7 QB Q VSS VSS nfet w=250n l=28n
MP7 QB Q VDD VDD pfet w=500n l=28n
* Slave feedback (weak): Q -> m2
MN8 m2 Q VSS VSS nfet w=100n l=28n
MP8 m2 Q VDD VDD pfet w=200n l=28n
.ends
