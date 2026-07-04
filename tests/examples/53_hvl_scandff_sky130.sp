* HVL scan D flip-flop with reset (SKY130) [n2s test case 53]
*
* === n2s test-case metadata ===
* Source:       sky130A PDK, libs.ref/sky130_fd_sc_hvl/spice/
*               sky130_fd_sc_hvl.spice
* Circuit type: Scan-mux D flip-flop, active-low reset, dual outputs
*               (sdfrbp_1) -- the largest flat transistor-level stdcell
*               in the corpus
* Devices:      42 X-FETs (g5v0d10v5 HV family)
* Notes:        Scan mux + master/slave latches + reset gating in one
*               flat cell; stresses gate extraction adjacency (below the
*               60-device collapse gate, so patterns + HAC handle it).
* Added:        2026-07-04  (batch 7)
* ===============================
.subckt sky130_fd_sc_hvl__sdfrbp_1 CLK D RESET_B SCD SCE VGND VNB VPB VPWR Q Q_N
X0 a_3098_107# a_2624_107# a_2841_81# VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X1 VPWR SCD a_794_655# VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=500000u
X2 a_1999_126# a_2014_537# a_2141_126# VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X3 VGND a_2624_107# Q_N VNB sky130_fd_pr__nfet_g5v0d10v5 w=750000u l=500000u
X4 a_1972_659# a_2014_537# VPWR VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=500000u
X5 a_2871_543# a_2841_81# VPWR VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=500000u
X6 VGND CLK a_1290_126# VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X7 a_2799_107# a_2841_81# VGND VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X8 a_2624_107# a_1290_126# a_2799_107# VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X9 VPWR a_1290_126# a_1569_126# VPB sky130_fd_pr__pfet_g5v0d10v5 w=750000u l=500000u
X10 a_339_655# D a_496_655# VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=500000u
X11 a_1816_659# a_1290_126# a_1972_659# VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=500000u
X12 VPWR a_1816_659# a_2014_537# VPB sky130_fd_pr__pfet_g5v0d10v5 w=1e+06u l=500000u
X13 VPWR RESET_B a_2841_81# VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=500000u
X14 a_794_655# a_222_131# a_339_655# VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=500000u
X15 a_339_655# SCE a_816_107# VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X16 VPWR CLK a_1290_126# VPB sky130_fd_pr__pfet_g5v0d10v5 w=750000u l=500000u
X17 VPWR RESET_B a_1816_659# VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=500000u
X18 VGND a_1816_659# a_2014_537# VNB sky130_fd_pr__nfet_g5v0d10v5 w=750000u l=500000u
X19 VGND SCE a_222_131# VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X20 a_2841_81# a_2624_107# VPWR VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=500000u
X21 a_3613_443# a_2624_107# VGND VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X22 a_2141_126# RESET_B VGND VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X23 a_361_107# D a_518_107# VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X24 a_496_655# SCE VPWR VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=500000u
X25 a_339_655# RESET_B VPWR VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=500000u
X26 VGND a_1290_126# a_1569_126# VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X27 a_2014_537# a_1569_126# a_2624_107# VNB sky130_fd_pr__nfet_g5v0d10v5 w=750000u l=500000u
X28 a_2624_107# a_1569_126# a_2871_543# VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=500000u
X29 a_518_107# a_222_131# a_339_655# VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X30 a_2014_537# a_1290_126# a_2624_107# VPB sky130_fd_pr__pfet_g5v0d10v5 w=1e+06u l=500000u
X31 a_361_107# RESET_B VGND VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X32 VGND RESET_B a_3098_107# VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X33 a_1816_659# a_1569_126# a_1999_126# VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X34 a_3613_443# a_2624_107# VPWR VPB sky130_fd_pr__pfet_g5v0d10v5 w=750000u l=500000u
X35 VPWR a_3613_443# Q VPB sky130_fd_pr__pfet_g5v0d10v5 w=1.5e+06u l=500000u
X36 VPWR SCE a_222_131# VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=500000u
X37 a_339_655# a_1290_126# a_1816_659# VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
X38 VPWR a_2624_107# Q_N VPB sky130_fd_pr__pfet_g5v0d10v5 w=1.5e+06u l=500000u
X39 VGND a_3613_443# Q VNB sky130_fd_pr__nfet_g5v0d10v5 w=750000u l=500000u
X40 a_339_655# a_1569_126# a_1816_659# VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=500000u
X41 a_816_107# SCD a_361_107# VNB sky130_fd_pr__nfet_g5v0d10v5 w=420000u l=500000u
.ends
