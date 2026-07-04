* SkyWater HD standard cell: D flip-flop (dfxtp_1) [n2s test case 37]
*
* === n2s test-case metadata ===
* Source:       sky130A PDK, libs.ref/sky130_fd_sc_hd/spice/sky130_fd_sc_hd.spice
*               (Google/SkyWater-authored cell netlist -- first test case
*               from OUTSIDE the myadc repo)
* Circuit type: Transmission-gate master-slave DFF, foundry stdcell
* Devices:      24 -- all X instances of sky130_fd_pr__*fet primitives
*               (12 pfet_01v8_hvt, 12 nfet_01v8); exercises the X-FET
*               port-semantics inference on a foreign netlist
* Notes:        Rails are VPWR/VGND with VPB/VNB bulk rails -- NONE of
*               these are in the hardcoded power-net list (audit item C1
*               territory: rail names the tool has never seen).
*               Extraction-style internal net names (a_891_413#).
* Added:        2026-07-04  (batch 3: non-myadc sources)
* ===============================
.subckt sky130_fd_sc_hd__dfxtp_1 CLK D VGND VNB VPB VPWR Q
X0 a_891_413# a_193_47# a_975_413# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X1 a_1059_315# a_891_413# VGND VNB sky130_fd_pr__nfet_01v8 w=650000u l=150000u
X2 a_466_413# a_27_47# a_561_413# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X3 a_634_159# a_27_47# a_891_413# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X4 a_381_47# a_193_47# a_466_413# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X5 VPWR D a_381_47# VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X6 VPWR a_466_413# a_634_159# VPB sky130_fd_pr__pfet_01v8_hvt w=750000u l=150000u
X7 VGND a_466_413# a_634_159# VNB sky130_fd_pr__nfet_01v8 w=640000u l=150000u
X8 a_1017_47# a_1059_315# VGND VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X9 a_1059_315# a_891_413# VPWR VPB sky130_fd_pr__pfet_01v8_hvt w=1e+06u l=150000u
X10 a_561_413# a_634_159# VPWR VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X11 VPWR a_1059_315# Q VPB sky130_fd_pr__pfet_01v8_hvt w=1e+06u l=150000u
X12 a_891_413# a_27_47# a_1017_47# VNB sky130_fd_pr__special_nfet_01v8 w=360000u l=150000u
X13 a_634_159# a_193_47# a_891_413# VNB sky130_fd_pr__special_nfet_01v8 w=360000u l=150000u
X14 a_592_47# a_634_159# VGND VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X15 a_466_413# a_193_47# a_592_47# VNB sky130_fd_pr__special_nfet_01v8 w=360000u l=150000u
X16 VGND a_27_47# a_193_47# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X17 a_381_47# a_27_47# a_466_413# VNB sky130_fd_pr__special_nfet_01v8 w=360000u l=150000u
X18 a_27_47# CLK VGND VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X19 a_27_47# CLK VPWR VPB sky130_fd_pr__pfet_01v8_hvt w=640000u l=150000u
X20 VPWR a_27_47# a_193_47# VPB sky130_fd_pr__pfet_01v8_hvt w=640000u l=150000u
X21 VGND D a_381_47# VNB sky130_fd_pr__nfet_01v8 w=420000u l=150000u
X22 a_975_413# a_1059_315# VPWR VPB sky130_fd_pr__pfet_01v8_hvt w=420000u l=150000u
X23 VGND a_1059_315# Q VNB sky130_fd_pr__nfet_01v8 w=650000u l=150000u
.ends
