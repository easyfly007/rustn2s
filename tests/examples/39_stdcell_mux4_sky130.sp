* SkyWater HD standard cell: 4-to-1 mux (mux4_1) [n2s test case 39]
*
* === n2s test-case metadata ===
* Source:       sky130A PDK, libs.ref/sky130_fd_sc_hd/spice/sky130_fd_sc_hd.spice
* Circuit type: Transmission-gate 4:1 multiplexer tree, foundry stdcell
* Devices:      26 -- all X instances of sky130_fd_pr__*fet primitives
* Notes:        TG tree topology: many pass-gates whose drain/source both
*               carry signals (no rail anchor) -- hard case for signal-flow
*               direction inference. VPWR/VGND/VPB/VNB rails.
* Added:        2026-07-04  (batch 3: non-myadc sources)
* ===============================
.subckt sky130_fd_sc_hd__mux4_1 A0 A1 A2 A3 S0 S1 VGND VNB VPB VPWR X
X0 a_277_47# S1 a_1478_413# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X1 a_757_363# S0 a_750_97# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X2 a_668_97# S0 a_750_97# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X3 a_750_97# S1 a_1478_413# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X4 VPWR A2 a_757_363# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X5 a_277_47# S0 a_27_47# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X6 a_1478_413# a_1290_413# a_277_47# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X7 a_277_47# S0 a_193_413# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X8 a_923_363# A3 VPWR VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X9 VPWR a_1478_413# X VPB sky130_fd_pr__pfet_01v8_hvt w=1e+06u l=150000u
X10 a_27_413# a_247_21# a_277_47# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X11 a_668_97# A3 VGND VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X12 VGND a_1478_413# X VNB sky130_fd_pr__nfet_01v8 w=650000u l=150000u
X13 a_193_47# a_247_21# a_277_47# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X14 a_247_21# S0 VGND VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X15 a_27_413# A1 VPWR VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X16 VPWR A0 a_193_413# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X17 a_1478_413# a_1290_413# a_750_97# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X18 a_247_21# S0 VPWR VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X19 a_750_97# a_247_21# a_923_363# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X20 VPWR S1 a_1290_413# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X21 VGND A0 a_193_47# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X22 a_27_47# A1 VGND VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X23 VGND A2 a_834_97# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X24 a_750_97# a_247_21# a_834_97# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X25 VGND S1 a_1290_413# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
.ends
