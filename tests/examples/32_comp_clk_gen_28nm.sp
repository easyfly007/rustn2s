* Flattened Comparator-Clock Generator (PTM 28nm) [n2s test case 32]
*
* === n2s test-case metadata ===
* Source:       myadc repo, ptm28nm/layout/tech/schematics/comp_clk_gen.spice
* Circuit type: 8-input NOR/NAND gate tree + output inverter, flattened to
*               transistors (4xNOR2 -> 2xNAND2 -> NOR2 -> INV)
* Devices:      30 total -- 30 MOSFET (16 nfet, 14 pfet), subckt-only netlist
* MOSFETs:      30; bare `nfet`/`pfet` model names (bulk-node inference)
* Notes:        Pure digital gate tree -- stresses HAC clustering and the
*               inverter pattern matcher on series-stack NOR/NAND shapes.
* Added:        2026-07-03  (real-world test-set enrichment, batch 2)
* ===============================
* Flattened COMP_CLK_GEN — 30 MOSFETs
* 4xNOR2 + 2xNAND2 + 1xNOR2 + 1xINV
* comp_clk = any phase active (NOR tree + buffer)

.subckt COMP_CLK_GEN ph0 ph1 ph2 ph3 ph4 ph5 ph6 ph7 comp_clk VDD VSS

* NOR2(ph0, ph1) -> n01
M_NOR01_P1 n01 ph0 NOR01_mid VDD pfet w=1000n l=28n
M_NOR01_P2 NOR01_mid ph1 VDD VDD pfet w=1000n l=28n
M_NOR01_N1 n01 ph0 VSS VSS nfet w=250n l=28n
M_NOR01_N2 n01 ph1 VSS VSS nfet w=250n l=28n

* NOR2(ph2, ph3) -> n23
M_NOR23_P1 n23 ph2 NOR23_mid VDD pfet w=1000n l=28n
M_NOR23_P2 NOR23_mid ph3 VDD VDD pfet w=1000n l=28n
M_NOR23_N1 n23 ph2 VSS VSS nfet w=250n l=28n
M_NOR23_N2 n23 ph3 VSS VSS nfet w=250n l=28n

* NOR2(ph4, ph5) -> n45
M_NOR45_P1 n45 ph4 NOR45_mid VDD pfet w=1000n l=28n
M_NOR45_P2 NOR45_mid ph5 VDD VDD pfet w=1000n l=28n
M_NOR45_N1 n45 ph4 VSS VSS nfet w=250n l=28n
M_NOR45_N2 n45 ph5 VSS VSS nfet w=250n l=28n

* NOR2(ph6, ph7) -> n67
M_NOR67_P1 n67 ph6 NOR67_mid VDD pfet w=1000n l=28n
M_NOR67_P2 NOR67_mid ph7 VDD VDD pfet w=1000n l=28n
M_NOR67_N1 n67 ph6 VSS VSS nfet w=250n l=28n
M_NOR67_N2 n67 ph7 VSS VSS nfet w=250n l=28n

* NAND2(n01, n23) -> na
M_NANDA_P1 na n01 VDD VDD pfet w=500n l=28n
M_NANDA_P2 na n23 VDD VDD pfet w=500n l=28n
M_NANDA_N1 na n01 NANDA_mid VSS nfet w=500n l=28n
M_NANDA_N2 NANDA_mid n23 VSS VSS nfet w=500n l=28n

* NAND2(n45, n67) -> nb
M_NANDB_P1 nb n45 VDD VDD pfet w=500n l=28n
M_NANDB_P2 nb n67 VDD VDD pfet w=500n l=28n
M_NANDB_N1 nb n45 NANDB_mid VSS nfet w=500n l=28n
M_NANDB_N2 NANDB_mid n67 VSS VSS nfet w=500n l=28n

* NOR2(na, nb) -> comp_clk_pre
M_NORAB_P1 comp_clk_pre na NORAB_mid VDD pfet w=1000n l=28n
M_NORAB_P2 NORAB_mid nb VDD VDD pfet w=1000n l=28n
M_NORAB_N1 comp_clk_pre na VSS VSS nfet w=250n l=28n
M_NORAB_N2 comp_clk_pre nb VSS VSS nfet w=250n l=28n

* INV(comp_clk_pre) -> comp_clk
M_INV_CC1_P comp_clk comp_clk_pre VDD VDD pfet w=500n l=28n
M_INV_CC1_N comp_clk comp_clk_pre VSS VSS nfet w=250n l=28n

.ends COMP_CLK_GEN
