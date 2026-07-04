* SkyWater HD standard cell: full adder (fa_1) [n2s test case 38]
*
* === n2s test-case metadata ===
* Source:       sky130A PDK, libs.ref/sky130_fd_sc_hd/spice/sky130_fd_sc_hd.spice
* Circuit type: Single-bit full adder (majority + XOR trees), foundry stdcell
* Devices:      28 -- all X instances of sky130_fd_pr__*fet primitives
* Notes:        Deep series stacks (3-high) in the carry/sum trees --
*               stresses HAC clustering and stack pitch on X boxes.
*               VPWR/VGND/VPB/VNB rails (not in the power-name list).
* Added:        2026-07-04  (batch 3: non-myadc sources)
* ===============================
.subckt sky130_fd_sc_hd__fa_1 A B CIN VGND VNB VPB VPWR COUT SUM
X0 VGND A a_208_47# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X1 VGND B a_382_47# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X2 a_1163_413# A VPWR VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X3 a_208_413# B a_76_199# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X4 a_382_413# A VPWR VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X5 VPWR A a_738_413# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X6 a_76_199# CIN a_382_413# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X7 a_738_413# CIN VPWR VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X8 a_995_47# CIN a_1091_413# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X9 VPWR B a_382_413# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X10 a_1091_413# B a_1163_413# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X11 VGND A a_738_47# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X12 a_382_47# A VGND VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X13 VGND B a_738_47# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X14 VPWR a_995_47# SUM VPB sky130_fd_pr__pfet_01v8_hvt w=1e+06u l=150000u
X15 a_738_47# CIN VGND VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X16 a_738_47# a_76_199# a_995_47# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X17 a_76_199# CIN a_382_47# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X18 a_1091_47# B a_1163_47# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X19 a_738_413# a_76_199# a_995_47# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X20 VGND a_995_47# SUM VNB sky130_fd_pr__nfet_01v8 w=650000u l=150000u
X21 VPWR B a_738_413# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X22 a_1163_47# A VGND VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X23 VPWR A a_208_413# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X24 COUT a_76_199# VGND VNB sky130_fd_pr__nfet_01v8 w=650000u l=150000u
X25 COUT a_76_199# VPWR VPB sky130_fd_pr__pfet_01v8_hvt w=1e+06u l=150000u
X26 a_208_47# B a_76_199# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X27 a_995_47# CIN a_1091_47# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
.ends
