* HVL low-to-high level shifter (SKY130) [n2s test case 52]
*
* === n2s test-case metadata ===
* Source:       sky130A PDK, libs.ref/sky130_fd_sc_hvl/spice/
*               sky130_fd_sc_hvl.spice (SkyWater high-voltage library)
* Circuit type: 1.8V -> 5V level-shifting buffer (cross-coupled HV pull-
*               ups + LV input stage), the hvl library's signature cell
* Devices:      10 X-FETs, MIXED families: sky130_fd_pr__nfet_01v8 (LV)
*               and *fet_g5v0d10v5 (HV)
* Notes:        Carries an LVPWR port -- a supply rail NOT in the builtin
*               list and not a V-source terminal: a live C1 specimen from
*               a real library. Measured 2026-07-04: unhinted it still
*               passes Tier 1 (LVPWR feeds only 2 devices here; quality
*               0.380 vs 0.388 hinted) -- mild, unlike case 40's failure.
*               The directive stays as the correct semantic declaration.
* Added:        2026-07-04  (batch 7)
* ===============================

* n2s: power_net LVPWR
.subckt sky130_fd_sc_hvl__lsbuflv2hv_1 A LVPWR VGND VNB VPB VPWR X
X0 VGND a_404_1133# a_504_1221# VNB sky130_fd_pr__nfet_g5v0d10v5 w=1.5e+06u l=500000u
X1 a_504_1221# a_404_1133# VGND VNB sky130_fd_pr__nfet_g5v0d10v5 w=1.5e+06u l=500000u
X2 X a_1711_885# VPWR VPB sky130_fd_pr__pfet_g5v0d10v5 w=1.5e+06u l=500000u
X3 X a_1711_885# VGND VNB sky130_fd_pr__nfet_g5v0d10v5 w=750000u l=500000u
X4 VGND A a_404_1133# VNB sky130_fd_pr__nfet_01v8 w=840000u l=150000u
X5 a_1197_107# a_772_151# VGND VNB sky130_fd_pr__nfet_g5v0d10v5 w=1.5e+06u l=500000u
X6 VPWR a_1197_107# a_504_1221# VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=1e+06u
X7 a_504_1221# a_404_1133# VGND VNB sky130_fd_pr__nfet_g5v0d10v5 w=1.5e+06u l=500000u
X8 a_1197_107# a_772_151# VGND VNB sky130_fd_pr__nfet_g5v0d10v5 w=1.5e+06u l=500000u
X9 a_772_151# a_404_1133# VGND VNB sky130_fd_pr__nfet_01v8 w=840000u l=150000u
X10 a_504_1221# a_404_1133# VGND VNB sky130_fd_pr__nfet_g5v0d10v5 w=1.5e+06u l=500000u
X11 VGND a_404_1133# a_504_1221# VNB sky130_fd_pr__nfet_g5v0d10v5 w=1.5e+06u l=500000u
X12 LVPWR A a_404_1133# LVPWR sky130_fd_pr__pfet_01v8_hvt w=840000u l=150000u
X13 VGND a_772_151# a_1197_107# VNB sky130_fd_pr__nfet_g5v0d10v5 w=1.5e+06u l=500000u
X14 VPWR a_504_1221# a_1711_885# VPB sky130_fd_pr__pfet_g5v0d10v5 w=1.5e+06u l=500000u
X15 VGND a_504_1221# a_1711_885# VNB sky130_fd_pr__nfet_g5v0d10v5 w=750000u l=500000u
X16 VGND a_772_151# a_1197_107# VNB sky130_fd_pr__nfet_g5v0d10v5 w=1.5e+06u l=500000u
X17 a_772_151# a_404_1133# LVPWR LVPWR sky130_fd_pr__pfet_01v8_hvt w=840000u l=150000u
X18 a_1197_107# a_772_151# VGND VNB sky130_fd_pr__nfet_g5v0d10v5 w=1.5e+06u l=500000u
X19 VPWR a_504_1221# a_1197_107# VPB sky130_fd_pr__pfet_g5v0d10v5 w=420000u l=1e+06u
.ends
