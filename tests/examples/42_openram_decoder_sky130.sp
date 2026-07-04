* OpenRAM hierarchical row decoder, 131 gate instances (SKY130) [n2s test case 42]
*
* === n2s test-case metadata ===
* Source:       sky130A PDK, libs.ref/sky130_sram_macros/spice/
*               sram_1rw1r_32_256_8_sky130.spice (OpenRAM-generated;
*               third netlist author in the corpus)
* Circuit type: 256-row hierarchical address decoder: 3x 3:8 predecoders
*               feeding 256 and3 gates, plus inverter banks
* Devices:      131 top-level X instances (and3_dec / pinvbuf / pre3x8
*               cells, all locally defined down to FET primitives)
* Notes:        SCALE case from a new author -- prerequisite for scale
*               Phase 1 (docs/scale_placement.md). Deep local hierarchy:
*               8 subckt defs, leaf devices are sky130_fd_pr__*fet.
*               Subckt-only netlist; target subckt placed first.
* Added:        2026-07-04  (batch 4: scale prerequisites)
* ===============================
.SUBCKT hierarchical_decoder addr_0 addr_1 addr_2 addr_3 addr_4 addr_5 addr_6 decode_0 decode_1 decode_2 decode_3 decode_4 decode_5 decode_6 decode_7 decode_8 decode_9 decode_10 decode_11 decode_12 decode_13 decode_14 decode_15 decode_16 decode_17 decode_18 decode_19 decode_20 decode_21 decode_22 decode_23 decode_24 decode_25 decode_26 decode_27 decode_28 decode_29 decode_30 decode_31 decode_32 decode_33 decode_34 decode_35 decode_36 decode_37 decode_38 decode_39 decode_40 decode_41 decode_42 decode_43 decode_44 decode_45 decode_46 decode_47 decode_48 decode_49 decode_50 decode_51 decode_52 decode_53 decode_54 decode_55 decode_56 decode_57 decode_58 decode_59 decode_60 decode_61 decode_62 decode_63 decode_64 decode_65 decode_66 decode_67 decode_68 decode_69 decode_70 decode_71 decode_72 decode_73 decode_74 decode_75 decode_76 decode_77 decode_78 decode_79 decode_80 decode_81 decode_82 decode_83 decode_84 decode_85 decode_86 decode_87 decode_88 decode_89 decode_90 decode_91 decode_92 decode_93 decode_94 decode_95 decode_96 decode_97 decode_98 decode_99 decode_100 decode_101 decode_102 decode_103 decode_104 decode_105 decode_106 decode_107 decode_108 decode_109 decode_110 decode_111 decode_112 decode_113 decode_114 decode_115 decode_116 decode_117 decode_118 decode_119 decode_120 decode_121 decode_122 decode_123 decode_124 decode_125 decode_126 decode_127 vdd gnd
* INPUT : addr_0 
* INPUT : addr_1 
* INPUT : addr_2 
* INPUT : addr_3 
* INPUT : addr_4 
* INPUT : addr_5 
* INPUT : addr_6 
* OUTPUT: decode_0 
* OUTPUT: decode_1 
* OUTPUT: decode_2 
* OUTPUT: decode_3 
* OUTPUT: decode_4 
* OUTPUT: decode_5 
* OUTPUT: decode_6 
* OUTPUT: decode_7 
* OUTPUT: decode_8 
* OUTPUT: decode_9 
* OUTPUT: decode_10 
* OUTPUT: decode_11 
* OUTPUT: decode_12 
* OUTPUT: decode_13 
* OUTPUT: decode_14 
* OUTPUT: decode_15 
* OUTPUT: decode_16 
* OUTPUT: decode_17 
* OUTPUT: decode_18 
* OUTPUT: decode_19 
* OUTPUT: decode_20 
* OUTPUT: decode_21 
* OUTPUT: decode_22 
* OUTPUT: decode_23 
* OUTPUT: decode_24 
* OUTPUT: decode_25 
* OUTPUT: decode_26 
* OUTPUT: decode_27 
* OUTPUT: decode_28 
* OUTPUT: decode_29 
* OUTPUT: decode_30 
* OUTPUT: decode_31 
* OUTPUT: decode_32 
* OUTPUT: decode_33 
* OUTPUT: decode_34 
* OUTPUT: decode_35 
* OUTPUT: decode_36 
* OUTPUT: decode_37 
* OUTPUT: decode_38 
* OUTPUT: decode_39 
* OUTPUT: decode_40 
* OUTPUT: decode_41 
* OUTPUT: decode_42 
* OUTPUT: decode_43 
* OUTPUT: decode_44 
* OUTPUT: decode_45 
* OUTPUT: decode_46 
* OUTPUT: decode_47 
* OUTPUT: decode_48 
* OUTPUT: decode_49 
* OUTPUT: decode_50 
* OUTPUT: decode_51 
* OUTPUT: decode_52 
* OUTPUT: decode_53 
* OUTPUT: decode_54 
* OUTPUT: decode_55 
* OUTPUT: decode_56 
* OUTPUT: decode_57 
* OUTPUT: decode_58 
* OUTPUT: decode_59 
* OUTPUT: decode_60 
* OUTPUT: decode_61 
* OUTPUT: decode_62 
* OUTPUT: decode_63 
* OUTPUT: decode_64 
* OUTPUT: decode_65 
* OUTPUT: decode_66 
* OUTPUT: decode_67 
* OUTPUT: decode_68 
* OUTPUT: decode_69 
* OUTPUT: decode_70 
* OUTPUT: decode_71 
* OUTPUT: decode_72 
* OUTPUT: decode_73 
* OUTPUT: decode_74 
* OUTPUT: decode_75 
* OUTPUT: decode_76 
* OUTPUT: decode_77 
* OUTPUT: decode_78 
* OUTPUT: decode_79 
* OUTPUT: decode_80 
* OUTPUT: decode_81 
* OUTPUT: decode_82 
* OUTPUT: decode_83 
* OUTPUT: decode_84 
* OUTPUT: decode_85 
* OUTPUT: decode_86 
* OUTPUT: decode_87 
* OUTPUT: decode_88 
* OUTPUT: decode_89 
* OUTPUT: decode_90 
* OUTPUT: decode_91 
* OUTPUT: decode_92 
* OUTPUT: decode_93 
* OUTPUT: decode_94 
* OUTPUT: decode_95 
* OUTPUT: decode_96 
* OUTPUT: decode_97 
* OUTPUT: decode_98 
* OUTPUT: decode_99 
* OUTPUT: decode_100 
* OUTPUT: decode_101 
* OUTPUT: decode_102 
* OUTPUT: decode_103 
* OUTPUT: decode_104 
* OUTPUT: decode_105 
* OUTPUT: decode_106 
* OUTPUT: decode_107 
* OUTPUT: decode_108 
* OUTPUT: decode_109 
* OUTPUT: decode_110 
* OUTPUT: decode_111 
* OUTPUT: decode_112 
* OUTPUT: decode_113 
* OUTPUT: decode_114 
* OUTPUT: decode_115 
* OUTPUT: decode_116 
* OUTPUT: decode_117 
* OUTPUT: decode_118 
* OUTPUT: decode_119 
* OUTPUT: decode_120 
* OUTPUT: decode_121 
* OUTPUT: decode_122 
* OUTPUT: decode_123 
* OUTPUT: decode_124 
* OUTPUT: decode_125 
* OUTPUT: decode_126 
* OUTPUT: decode_127 
* POWER : vdd 
* GROUND: gnd 
Xpre_0 addr_0 addr_1 out_0 out_1 out_2 out_3 vdd gnd hierarchical_predecode2x4
Xpre_1 addr_2 addr_3 out_4 out_5 out_6 out_7 vdd gnd hierarchical_predecode2x4
Xpre3x8_0 addr_4 addr_5 addr_6 out_8 out_9 out_10 out_11 out_12 out_13 out_14 out_15 vdd gnd hierarchical_predecode3x8
XDEC_AND_0 out_0 out_4 out_8 decode_0 vdd gnd and3_dec
XDEC_AND_16 out_0 out_4 out_9 decode_16 vdd gnd and3_dec
XDEC_AND_32 out_0 out_4 out_10 decode_32 vdd gnd and3_dec
XDEC_AND_48 out_0 out_4 out_11 decode_48 vdd gnd and3_dec
XDEC_AND_64 out_0 out_4 out_12 decode_64 vdd gnd and3_dec
XDEC_AND_80 out_0 out_4 out_13 decode_80 vdd gnd and3_dec
XDEC_AND_96 out_0 out_4 out_14 decode_96 vdd gnd and3_dec
XDEC_AND_112 out_0 out_4 out_15 decode_112 vdd gnd and3_dec
XDEC_AND_4 out_0 out_5 out_8 decode_4 vdd gnd and3_dec
XDEC_AND_20 out_0 out_5 out_9 decode_20 vdd gnd and3_dec
XDEC_AND_36 out_0 out_5 out_10 decode_36 vdd gnd and3_dec
XDEC_AND_52 out_0 out_5 out_11 decode_52 vdd gnd and3_dec
XDEC_AND_68 out_0 out_5 out_12 decode_68 vdd gnd and3_dec
XDEC_AND_84 out_0 out_5 out_13 decode_84 vdd gnd and3_dec
XDEC_AND_100 out_0 out_5 out_14 decode_100 vdd gnd and3_dec
XDEC_AND_116 out_0 out_5 out_15 decode_116 vdd gnd and3_dec
XDEC_AND_8 out_0 out_6 out_8 decode_8 vdd gnd and3_dec
XDEC_AND_24 out_0 out_6 out_9 decode_24 vdd gnd and3_dec
XDEC_AND_40 out_0 out_6 out_10 decode_40 vdd gnd and3_dec
XDEC_AND_56 out_0 out_6 out_11 decode_56 vdd gnd and3_dec
XDEC_AND_72 out_0 out_6 out_12 decode_72 vdd gnd and3_dec
XDEC_AND_88 out_0 out_6 out_13 decode_88 vdd gnd and3_dec
XDEC_AND_104 out_0 out_6 out_14 decode_104 vdd gnd and3_dec
XDEC_AND_120 out_0 out_6 out_15 decode_120 vdd gnd and3_dec
XDEC_AND_12 out_0 out_7 out_8 decode_12 vdd gnd and3_dec
XDEC_AND_28 out_0 out_7 out_9 decode_28 vdd gnd and3_dec
XDEC_AND_44 out_0 out_7 out_10 decode_44 vdd gnd and3_dec
XDEC_AND_60 out_0 out_7 out_11 decode_60 vdd gnd and3_dec
XDEC_AND_76 out_0 out_7 out_12 decode_76 vdd gnd and3_dec
XDEC_AND_92 out_0 out_7 out_13 decode_92 vdd gnd and3_dec
XDEC_AND_108 out_0 out_7 out_14 decode_108 vdd gnd and3_dec
XDEC_AND_124 out_0 out_7 out_15 decode_124 vdd gnd and3_dec
XDEC_AND_1 out_1 out_4 out_8 decode_1 vdd gnd and3_dec
XDEC_AND_17 out_1 out_4 out_9 decode_17 vdd gnd and3_dec
XDEC_AND_33 out_1 out_4 out_10 decode_33 vdd gnd and3_dec
XDEC_AND_49 out_1 out_4 out_11 decode_49 vdd gnd and3_dec
XDEC_AND_65 out_1 out_4 out_12 decode_65 vdd gnd and3_dec
XDEC_AND_81 out_1 out_4 out_13 decode_81 vdd gnd and3_dec
XDEC_AND_97 out_1 out_4 out_14 decode_97 vdd gnd and3_dec
XDEC_AND_113 out_1 out_4 out_15 decode_113 vdd gnd and3_dec
XDEC_AND_5 out_1 out_5 out_8 decode_5 vdd gnd and3_dec
XDEC_AND_21 out_1 out_5 out_9 decode_21 vdd gnd and3_dec
XDEC_AND_37 out_1 out_5 out_10 decode_37 vdd gnd and3_dec
XDEC_AND_53 out_1 out_5 out_11 decode_53 vdd gnd and3_dec
XDEC_AND_69 out_1 out_5 out_12 decode_69 vdd gnd and3_dec
XDEC_AND_85 out_1 out_5 out_13 decode_85 vdd gnd and3_dec
XDEC_AND_101 out_1 out_5 out_14 decode_101 vdd gnd and3_dec
XDEC_AND_117 out_1 out_5 out_15 decode_117 vdd gnd and3_dec
XDEC_AND_9 out_1 out_6 out_8 decode_9 vdd gnd and3_dec
XDEC_AND_25 out_1 out_6 out_9 decode_25 vdd gnd and3_dec
XDEC_AND_41 out_1 out_6 out_10 decode_41 vdd gnd and3_dec
XDEC_AND_57 out_1 out_6 out_11 decode_57 vdd gnd and3_dec
XDEC_AND_73 out_1 out_6 out_12 decode_73 vdd gnd and3_dec
XDEC_AND_89 out_1 out_6 out_13 decode_89 vdd gnd and3_dec
XDEC_AND_105 out_1 out_6 out_14 decode_105 vdd gnd and3_dec
XDEC_AND_121 out_1 out_6 out_15 decode_121 vdd gnd and3_dec
XDEC_AND_13 out_1 out_7 out_8 decode_13 vdd gnd and3_dec
XDEC_AND_29 out_1 out_7 out_9 decode_29 vdd gnd and3_dec
XDEC_AND_45 out_1 out_7 out_10 decode_45 vdd gnd and3_dec
XDEC_AND_61 out_1 out_7 out_11 decode_61 vdd gnd and3_dec
XDEC_AND_77 out_1 out_7 out_12 decode_77 vdd gnd and3_dec
XDEC_AND_93 out_1 out_7 out_13 decode_93 vdd gnd and3_dec
XDEC_AND_109 out_1 out_7 out_14 decode_109 vdd gnd and3_dec
XDEC_AND_125 out_1 out_7 out_15 decode_125 vdd gnd and3_dec
XDEC_AND_2 out_2 out_4 out_8 decode_2 vdd gnd and3_dec
XDEC_AND_18 out_2 out_4 out_9 decode_18 vdd gnd and3_dec
XDEC_AND_34 out_2 out_4 out_10 decode_34 vdd gnd and3_dec
XDEC_AND_50 out_2 out_4 out_11 decode_50 vdd gnd and3_dec
XDEC_AND_66 out_2 out_4 out_12 decode_66 vdd gnd and3_dec
XDEC_AND_82 out_2 out_4 out_13 decode_82 vdd gnd and3_dec
XDEC_AND_98 out_2 out_4 out_14 decode_98 vdd gnd and3_dec
XDEC_AND_114 out_2 out_4 out_15 decode_114 vdd gnd and3_dec
XDEC_AND_6 out_2 out_5 out_8 decode_6 vdd gnd and3_dec
XDEC_AND_22 out_2 out_5 out_9 decode_22 vdd gnd and3_dec
XDEC_AND_38 out_2 out_5 out_10 decode_38 vdd gnd and3_dec
XDEC_AND_54 out_2 out_5 out_11 decode_54 vdd gnd and3_dec
XDEC_AND_70 out_2 out_5 out_12 decode_70 vdd gnd and3_dec
XDEC_AND_86 out_2 out_5 out_13 decode_86 vdd gnd and3_dec
XDEC_AND_102 out_2 out_5 out_14 decode_102 vdd gnd and3_dec
XDEC_AND_118 out_2 out_5 out_15 decode_118 vdd gnd and3_dec
XDEC_AND_10 out_2 out_6 out_8 decode_10 vdd gnd and3_dec
XDEC_AND_26 out_2 out_6 out_9 decode_26 vdd gnd and3_dec
XDEC_AND_42 out_2 out_6 out_10 decode_42 vdd gnd and3_dec
XDEC_AND_58 out_2 out_6 out_11 decode_58 vdd gnd and3_dec
XDEC_AND_74 out_2 out_6 out_12 decode_74 vdd gnd and3_dec
XDEC_AND_90 out_2 out_6 out_13 decode_90 vdd gnd and3_dec
XDEC_AND_106 out_2 out_6 out_14 decode_106 vdd gnd and3_dec
XDEC_AND_122 out_2 out_6 out_15 decode_122 vdd gnd and3_dec
XDEC_AND_14 out_2 out_7 out_8 decode_14 vdd gnd and3_dec
XDEC_AND_30 out_2 out_7 out_9 decode_30 vdd gnd and3_dec
XDEC_AND_46 out_2 out_7 out_10 decode_46 vdd gnd and3_dec
XDEC_AND_62 out_2 out_7 out_11 decode_62 vdd gnd and3_dec
XDEC_AND_78 out_2 out_7 out_12 decode_78 vdd gnd and3_dec
XDEC_AND_94 out_2 out_7 out_13 decode_94 vdd gnd and3_dec
XDEC_AND_110 out_2 out_7 out_14 decode_110 vdd gnd and3_dec
XDEC_AND_126 out_2 out_7 out_15 decode_126 vdd gnd and3_dec
XDEC_AND_3 out_3 out_4 out_8 decode_3 vdd gnd and3_dec
XDEC_AND_19 out_3 out_4 out_9 decode_19 vdd gnd and3_dec
XDEC_AND_35 out_3 out_4 out_10 decode_35 vdd gnd and3_dec
XDEC_AND_51 out_3 out_4 out_11 decode_51 vdd gnd and3_dec
XDEC_AND_67 out_3 out_4 out_12 decode_67 vdd gnd and3_dec
XDEC_AND_83 out_3 out_4 out_13 decode_83 vdd gnd and3_dec
XDEC_AND_99 out_3 out_4 out_14 decode_99 vdd gnd and3_dec
XDEC_AND_115 out_3 out_4 out_15 decode_115 vdd gnd and3_dec
XDEC_AND_7 out_3 out_5 out_8 decode_7 vdd gnd and3_dec
XDEC_AND_23 out_3 out_5 out_9 decode_23 vdd gnd and3_dec
XDEC_AND_39 out_3 out_5 out_10 decode_39 vdd gnd and3_dec
XDEC_AND_55 out_3 out_5 out_11 decode_55 vdd gnd and3_dec
XDEC_AND_71 out_3 out_5 out_12 decode_71 vdd gnd and3_dec
XDEC_AND_87 out_3 out_5 out_13 decode_87 vdd gnd and3_dec
XDEC_AND_103 out_3 out_5 out_14 decode_103 vdd gnd and3_dec
XDEC_AND_119 out_3 out_5 out_15 decode_119 vdd gnd and3_dec
XDEC_AND_11 out_3 out_6 out_8 decode_11 vdd gnd and3_dec
XDEC_AND_27 out_3 out_6 out_9 decode_27 vdd gnd and3_dec
XDEC_AND_43 out_3 out_6 out_10 decode_43 vdd gnd and3_dec
XDEC_AND_59 out_3 out_6 out_11 decode_59 vdd gnd and3_dec
XDEC_AND_75 out_3 out_6 out_12 decode_75 vdd gnd and3_dec
XDEC_AND_91 out_3 out_6 out_13 decode_91 vdd gnd and3_dec
XDEC_AND_107 out_3 out_6 out_14 decode_107 vdd gnd and3_dec
XDEC_AND_123 out_3 out_6 out_15 decode_123 vdd gnd and3_dec
XDEC_AND_15 out_3 out_7 out_8 decode_15 vdd gnd and3_dec
XDEC_AND_31 out_3 out_7 out_9 decode_31 vdd gnd and3_dec
XDEC_AND_47 out_3 out_7 out_10 decode_47 vdd gnd and3_dec
XDEC_AND_63 out_3 out_7 out_11 decode_63 vdd gnd and3_dec
XDEC_AND_79 out_3 out_7 out_12 decode_79 vdd gnd and3_dec
XDEC_AND_95 out_3 out_7 out_13 decode_95 vdd gnd and3_dec
XDEC_AND_111 out_3 out_7 out_14 decode_111 vdd gnd and3_dec
XDEC_AND_127 out_3 out_7 out_15 decode_127 vdd gnd and3_dec
.ENDS hierarchical_decoder
.SUBCKT pinv_dec A Z vdd gnd
* INPUT : A 
* OUTPUT: Z 
* POWER : vdd 
* GROUND: gnd 
Xpinv_pmos Z A vdd vdd sky130_fd_pr__pfet_01v8 m=1 w=1.12 l=0.15 pd=2.54 ps=2.54 as=0.42u ad=0.42u mult=1
Xpinv_nmos Z A gnd gnd sky130_fd_pr__special_nfet_01v8 m=1 w=0.36 l=0.15 pd=1.02 ps=1.02 as=0.14u ad=0.14u mult=1
.ENDS pinv_dec
.subckt nand2_dec A B Z vdd gnd

X1001 Z B vdd vdd sky130_fd_pr__pfet_01v8 W=1.12 L=0.15 m=1 mult=1
X1002 vdd A Z vdd sky130_fd_pr__pfet_01v8 W=1.12 L=0.15 m=1 mult=1
X1000 Z A a_n722_276# gnd sky130_fd_pr__nfet_01v8 W=0.74 L=0.15 m=1 mult=1
X1003 a_n722_276# B gnd gnd sky130_fd_pr__nfet_01v8 W=0.74 L=0.15 m=1 mult=1
.ends
.SUBCKT and2_dec A B Z vdd gnd
* INPUT : A 
* INPUT : B 
* OUTPUT: Z 
* POWER : vdd 
* GROUND: gnd 
* size: 1
Xpand2_dec_nand A B zb_int vdd gnd nand2_dec
Xpand2_dec_inv zb_int Z vdd gnd pinv_dec
.ENDS and2_dec
.SUBCKT hierarchical_predecode2x4 in_0 in_1 out_0 out_1 out_2 out_3 vdd gnd
* INPUT : in_0 
* INPUT : in_1 
* OUTPUT: out_0 
* OUTPUT: out_1 
* OUTPUT: out_2 
* OUTPUT: out_3 
* POWER : vdd 
* GROUND: gnd 
Xpre_inv_0 in_0 inbar_0 vdd gnd pinv_dec
Xpre_inv_1 in_1 inbar_1 vdd gnd pinv_dec
XXpre2x4_and_0 inbar_0 inbar_1 out_0 vdd gnd and2_dec
XXpre2x4_and_1 in_0 inbar_1 out_1 vdd gnd and2_dec
XXpre2x4_and_2 inbar_0 in_1 out_2 vdd gnd and2_dec
XXpre2x4_and_3 in_0 in_1 out_3 vdd gnd and2_dec
.ENDS hierarchical_predecode2x4
.subckt nand3_dec A B C Z vdd gnd

X1001 Z A a_n346_328# gnd sky130_fd_pr__nfet_01v8 W=0.74 L=0.15 m=1 mult=1
X1002 a_n346_256# C gnd gnd sky130_fd_pr__nfet_01v8 W=0.74 L=0.15 m=1 mult=1
X1003 a_n346_328# B a_n346_256# gnd sky130_fd_pr__nfet_01v8 W=0.74 L=0.15 m=1 mult=1
X1000 Z B vdd vdd sky130_fd_pr__pfet_01v8 W=1.12 L=0.15 m=1 mult=1
X1004 Z A vdd vdd sky130_fd_pr__pfet_01v8 W=1.12 L=0.15 m=1 mult=1
X1005 Z C vdd vdd sky130_fd_pr__pfet_01v8 W=1.12 L=0.15 m=1 mult=1
.ends
.SUBCKT and3_dec A B C Z vdd gnd
* INPUT : A 
* INPUT : B 
* INPUT : C 
* OUTPUT: Z 
* POWER : vdd 
* GROUND: gnd 
* size: 1
Xpand3_dec_nand A B C zb_int vdd gnd nand3_dec
Xpand3_dec_inv zb_int Z vdd gnd pinv_dec
.ENDS and3_dec
.SUBCKT hierarchical_predecode3x8 in_0 in_1 in_2 out_0 out_1 out_2 out_3 out_4 out_5 out_6 out_7 vdd gnd
* INPUT : in_0 
* INPUT : in_1 
* INPUT : in_2 
* OUTPUT: out_0 
* OUTPUT: out_1 
* OUTPUT: out_2 
* OUTPUT: out_3 
* OUTPUT: out_4 
* OUTPUT: out_5 
* OUTPUT: out_6 
* OUTPUT: out_7 
* POWER : vdd 
* GROUND: gnd 
Xpre_inv_0 in_0 inbar_0 vdd gnd pinv_dec
Xpre_inv_1 in_1 inbar_1 vdd gnd pinv_dec
Xpre_inv_2 in_2 inbar_2 vdd gnd pinv_dec
XXpre3x8_and_0 inbar_0 inbar_1 inbar_2 out_0 vdd gnd and3_dec
XXpre3x8_and_1 in_0 inbar_1 inbar_2 out_1 vdd gnd and3_dec
XXpre3x8_and_2 inbar_0 in_1 inbar_2 out_2 vdd gnd and3_dec
XXpre3x8_and_3 in_0 in_1 inbar_2 out_3 vdd gnd and3_dec
XXpre3x8_and_4 inbar_0 inbar_1 in_2 out_4 vdd gnd and3_dec
XXpre3x8_and_5 in_0 inbar_1 in_2 out_5 vdd gnd and3_dec
XXpre3x8_and_6 inbar_0 in_1 in_2 out_6 vdd gnd and3_dec
XXpre3x8_and_7 in_0 in_1 in_2 out_7 vdd gnd and3_dec
.ENDS hierarchical_predecode3x8
