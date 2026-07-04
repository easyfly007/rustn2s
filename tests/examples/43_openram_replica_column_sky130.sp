* OpenRAM replica bitcell column, 130 instances (SKY130) [n2s test case 43]
*
* === n2s test-case metadata ===
* Source:       sky130A PDK, libs.ref/sky130_sram_macros/spice/
*               sram_1rw1r_32_256_8_sky130.spice (OpenRAM-generated)
* Circuit type: SRAM replica column: 130 dual-port bitcell instances
*               (openram_dp_cell / _replica variants) sharing bitlines
* Devices:      130 top-level X instances; leaf devices are
*               sky130_fd_pr__special_*fet latch/pass primitives
* Notes:        SCALE case, extreme repetition: one cell repeated 130x
*               on shared vertical bitline nets (bl/br) -- the exact
*               opposite topology of case 42's decoder tree. Stresses
*               huge-fanout net handling and repetitive-array layout.
* Added:        2026-07-04  (batch 4: scale prerequisites)
* ===============================
.SUBCKT replica_column bl0_0 br0_0 bl1_0 br1_0 wl0_0 wl1_0 wl0_1 wl1_1 wl0_2 wl1_2 wl0_3 wl1_3 wl0_4 wl1_4 wl0_5 wl1_5 wl0_6 wl1_6 wl0_7 wl1_7 wl0_8 wl1_8 wl0_9 wl1_9 wl0_10 wl1_10 wl0_11 wl1_11 wl0_12 wl1_12 wl0_13 wl1_13 wl0_14 wl1_14 wl0_15 wl1_15 wl0_16 wl1_16 wl0_17 wl1_17 wl0_18 wl1_18 wl0_19 wl1_19 wl0_20 wl1_20 wl0_21 wl1_21 wl0_22 wl1_22 wl0_23 wl1_23 wl0_24 wl1_24 wl0_25 wl1_25 wl0_26 wl1_26 wl0_27 wl1_27 wl0_28 wl1_28 wl0_29 wl1_29 wl0_30 wl1_30 wl0_31 wl1_31 wl0_32 wl1_32 wl0_33 wl1_33 wl0_34 wl1_34 wl0_35 wl1_35 wl0_36 wl1_36 wl0_37 wl1_37 wl0_38 wl1_38 wl0_39 wl1_39 wl0_40 wl1_40 wl0_41 wl1_41 wl0_42 wl1_42 wl0_43 wl1_43 wl0_44 wl1_44 wl0_45 wl1_45 wl0_46 wl1_46 wl0_47 wl1_47 wl0_48 wl1_48 wl0_49 wl1_49 wl0_50 wl1_50 wl0_51 wl1_51 wl0_52 wl1_52 wl0_53 wl1_53 wl0_54 wl1_54 wl0_55 wl1_55 wl0_56 wl1_56 wl0_57 wl1_57 wl0_58 wl1_58 wl0_59 wl1_59 wl0_60 wl1_60 wl0_61 wl1_61 wl0_62 wl1_62 wl0_63 wl1_63 wl0_64 wl1_64 wl0_65 wl1_65 wl0_66 wl1_66 wl0_67 wl1_67 wl0_68 wl1_68 wl0_69 wl1_69 wl0_70 wl1_70 wl0_71 wl1_71 wl0_72 wl1_72 wl0_73 wl1_73 wl0_74 wl1_74 wl0_75 wl1_75 wl0_76 wl1_76 wl0_77 wl1_77 wl0_78 wl1_78 wl0_79 wl1_79 wl0_80 wl1_80 wl0_81 wl1_81 wl0_82 wl1_82 wl0_83 wl1_83 wl0_84 wl1_84 wl0_85 wl1_85 wl0_86 wl1_86 wl0_87 wl1_87 wl0_88 wl1_88 wl0_89 wl1_89 wl0_90 wl1_90 wl0_91 wl1_91 wl0_92 wl1_92 wl0_93 wl1_93 wl0_94 wl1_94 wl0_95 wl1_95 wl0_96 wl1_96 wl0_97 wl1_97 wl0_98 wl1_98 wl0_99 wl1_99 wl0_100 wl1_100 wl0_101 wl1_101 wl0_102 wl1_102 wl0_103 wl1_103 wl0_104 wl1_104 wl0_105 wl1_105 wl0_106 wl1_106 wl0_107 wl1_107 wl0_108 wl1_108 wl0_109 wl1_109 wl0_110 wl1_110 wl0_111 wl1_111 wl0_112 wl1_112 wl0_113 wl1_113 wl0_114 wl1_114 wl0_115 wl1_115 wl0_116 wl1_116 wl0_117 wl1_117 wl0_118 wl1_118 wl0_119 wl1_119 wl0_120 wl1_120 wl0_121 wl1_121 wl0_122 wl1_122 wl0_123 wl1_123 wl0_124 wl1_124 wl0_125 wl1_125 wl0_126 wl1_126 wl0_127 wl1_127 wl0_128 wl1_128 wl0_129 wl1_129 wl0_130 wl1_130 wl0_131 wl1_131 vdd gnd
* OUTPUT: bl0_0 
* OUTPUT: br0_0 
* OUTPUT: bl1_0 
* OUTPUT: br1_0 
* INPUT : wl0_0 
* INPUT : wl1_0 
* INPUT : wl0_1 
* INPUT : wl1_1 
* INPUT : wl0_2 
* INPUT : wl1_2 
* INPUT : wl0_3 
* INPUT : wl1_3 
* INPUT : wl0_4 
* INPUT : wl1_4 
* INPUT : wl0_5 
* INPUT : wl1_5 
* INPUT : wl0_6 
* INPUT : wl1_6 
* INPUT : wl0_7 
* INPUT : wl1_7 
* INPUT : wl0_8 
* INPUT : wl1_8 
* INPUT : wl0_9 
* INPUT : wl1_9 
* INPUT : wl0_10 
* INPUT : wl1_10 
* INPUT : wl0_11 
* INPUT : wl1_11 
* INPUT : wl0_12 
* INPUT : wl1_12 
* INPUT : wl0_13 
* INPUT : wl1_13 
* INPUT : wl0_14 
* INPUT : wl1_14 
* INPUT : wl0_15 
* INPUT : wl1_15 
* INPUT : wl0_16 
* INPUT : wl1_16 
* INPUT : wl0_17 
* INPUT : wl1_17 
* INPUT : wl0_18 
* INPUT : wl1_18 
* INPUT : wl0_19 
* INPUT : wl1_19 
* INPUT : wl0_20 
* INPUT : wl1_20 
* INPUT : wl0_21 
* INPUT : wl1_21 
* INPUT : wl0_22 
* INPUT : wl1_22 
* INPUT : wl0_23 
* INPUT : wl1_23 
* INPUT : wl0_24 
* INPUT : wl1_24 
* INPUT : wl0_25 
* INPUT : wl1_25 
* INPUT : wl0_26 
* INPUT : wl1_26 
* INPUT : wl0_27 
* INPUT : wl1_27 
* INPUT : wl0_28 
* INPUT : wl1_28 
* INPUT : wl0_29 
* INPUT : wl1_29 
* INPUT : wl0_30 
* INPUT : wl1_30 
* INPUT : wl0_31 
* INPUT : wl1_31 
* INPUT : wl0_32 
* INPUT : wl1_32 
* INPUT : wl0_33 
* INPUT : wl1_33 
* INPUT : wl0_34 
* INPUT : wl1_34 
* INPUT : wl0_35 
* INPUT : wl1_35 
* INPUT : wl0_36 
* INPUT : wl1_36 
* INPUT : wl0_37 
* INPUT : wl1_37 
* INPUT : wl0_38 
* INPUT : wl1_38 
* INPUT : wl0_39 
* INPUT : wl1_39 
* INPUT : wl0_40 
* INPUT : wl1_40 
* INPUT : wl0_41 
* INPUT : wl1_41 
* INPUT : wl0_42 
* INPUT : wl1_42 
* INPUT : wl0_43 
* INPUT : wl1_43 
* INPUT : wl0_44 
* INPUT : wl1_44 
* INPUT : wl0_45 
* INPUT : wl1_45 
* INPUT : wl0_46 
* INPUT : wl1_46 
* INPUT : wl0_47 
* INPUT : wl1_47 
* INPUT : wl0_48 
* INPUT : wl1_48 
* INPUT : wl0_49 
* INPUT : wl1_49 
* INPUT : wl0_50 
* INPUT : wl1_50 
* INPUT : wl0_51 
* INPUT : wl1_51 
* INPUT : wl0_52 
* INPUT : wl1_52 
* INPUT : wl0_53 
* INPUT : wl1_53 
* INPUT : wl0_54 
* INPUT : wl1_54 
* INPUT : wl0_55 
* INPUT : wl1_55 
* INPUT : wl0_56 
* INPUT : wl1_56 
* INPUT : wl0_57 
* INPUT : wl1_57 
* INPUT : wl0_58 
* INPUT : wl1_58 
* INPUT : wl0_59 
* INPUT : wl1_59 
* INPUT : wl0_60 
* INPUT : wl1_60 
* INPUT : wl0_61 
* INPUT : wl1_61 
* INPUT : wl0_62 
* INPUT : wl1_62 
* INPUT : wl0_63 
* INPUT : wl1_63 
* INPUT : wl0_64 
* INPUT : wl1_64 
* INPUT : wl0_65 
* INPUT : wl1_65 
* INPUT : wl0_66 
* INPUT : wl1_66 
* INPUT : wl0_67 
* INPUT : wl1_67 
* INPUT : wl0_68 
* INPUT : wl1_68 
* INPUT : wl0_69 
* INPUT : wl1_69 
* INPUT : wl0_70 
* INPUT : wl1_70 
* INPUT : wl0_71 
* INPUT : wl1_71 
* INPUT : wl0_72 
* INPUT : wl1_72 
* INPUT : wl0_73 
* INPUT : wl1_73 
* INPUT : wl0_74 
* INPUT : wl1_74 
* INPUT : wl0_75 
* INPUT : wl1_75 
* INPUT : wl0_76 
* INPUT : wl1_76 
* INPUT : wl0_77 
* INPUT : wl1_77 
* INPUT : wl0_78 
* INPUT : wl1_78 
* INPUT : wl0_79 
* INPUT : wl1_79 
* INPUT : wl0_80 
* INPUT : wl1_80 
* INPUT : wl0_81 
* INPUT : wl1_81 
* INPUT : wl0_82 
* INPUT : wl1_82 
* INPUT : wl0_83 
* INPUT : wl1_83 
* INPUT : wl0_84 
* INPUT : wl1_84 
* INPUT : wl0_85 
* INPUT : wl1_85 
* INPUT : wl0_86 
* INPUT : wl1_86 
* INPUT : wl0_87 
* INPUT : wl1_87 
* INPUT : wl0_88 
* INPUT : wl1_88 
* INPUT : wl0_89 
* INPUT : wl1_89 
* INPUT : wl0_90 
* INPUT : wl1_90 
* INPUT : wl0_91 
* INPUT : wl1_91 
* INPUT : wl0_92 
* INPUT : wl1_92 
* INPUT : wl0_93 
* INPUT : wl1_93 
* INPUT : wl0_94 
* INPUT : wl1_94 
* INPUT : wl0_95 
* INPUT : wl1_95 
* INPUT : wl0_96 
* INPUT : wl1_96 
* INPUT : wl0_97 
* INPUT : wl1_97 
* INPUT : wl0_98 
* INPUT : wl1_98 
* INPUT : wl0_99 
* INPUT : wl1_99 
* INPUT : wl0_100 
* INPUT : wl1_100 
* INPUT : wl0_101 
* INPUT : wl1_101 
* INPUT : wl0_102 
* INPUT : wl1_102 
* INPUT : wl0_103 
* INPUT : wl1_103 
* INPUT : wl0_104 
* INPUT : wl1_104 
* INPUT : wl0_105 
* INPUT : wl1_105 
* INPUT : wl0_106 
* INPUT : wl1_106 
* INPUT : wl0_107 
* INPUT : wl1_107 
* INPUT : wl0_108 
* INPUT : wl1_108 
* INPUT : wl0_109 
* INPUT : wl1_109 
* INPUT : wl0_110 
* INPUT : wl1_110 
* INPUT : wl0_111 
* INPUT : wl1_111 
* INPUT : wl0_112 
* INPUT : wl1_112 
* INPUT : wl0_113 
* INPUT : wl1_113 
* INPUT : wl0_114 
* INPUT : wl1_114 
* INPUT : wl0_115 
* INPUT : wl1_115 
* INPUT : wl0_116 
* INPUT : wl1_116 
* INPUT : wl0_117 
* INPUT : wl1_117 
* INPUT : wl0_118 
* INPUT : wl1_118 
* INPUT : wl0_119 
* INPUT : wl1_119 
* INPUT : wl0_120 
* INPUT : wl1_120 
* INPUT : wl0_121 
* INPUT : wl1_121 
* INPUT : wl0_122 
* INPUT : wl1_122 
* INPUT : wl0_123 
* INPUT : wl1_123 
* INPUT : wl0_124 
* INPUT : wl1_124 
* INPUT : wl0_125 
* INPUT : wl1_125 
* INPUT : wl0_126 
* INPUT : wl1_126 
* INPUT : wl0_127 
* INPUT : wl1_127 
* INPUT : wl0_128 
* INPUT : wl1_128 
* INPUT : wl0_129 
* INPUT : wl1_129 
* INPUT : wl0_130 
* INPUT : wl1_130 
* INPUT : wl0_131 
* INPUT : wl1_131 
* POWER : vdd 
* GROUND: gnd 
Xrbc_1 bl0_0 br0_0 bl1_0 br1_0 wl0_1 wl1_1 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_2 bl0_0 br0_0 bl1_0 br1_0 wl0_2 wl1_2 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_3 bl0_0 br0_0 bl1_0 br1_0 wl0_3 wl1_3 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_4 bl0_0 br0_0 bl1_0 br1_0 wl0_4 wl1_4 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_5 bl0_0 br0_0 bl1_0 br1_0 wl0_5 wl1_5 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_6 bl0_0 br0_0 bl1_0 br1_0 wl0_6 wl1_6 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_7 bl0_0 br0_0 bl1_0 br1_0 wl0_7 wl1_7 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_8 bl0_0 br0_0 bl1_0 br1_0 wl0_8 wl1_8 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_9 bl0_0 br0_0 bl1_0 br1_0 wl0_9 wl1_9 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_10 bl0_0 br0_0 bl1_0 br1_0 wl0_10 wl1_10 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_11 bl0_0 br0_0 bl1_0 br1_0 wl0_11 wl1_11 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_12 bl0_0 br0_0 bl1_0 br1_0 wl0_12 wl1_12 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_13 bl0_0 br0_0 bl1_0 br1_0 wl0_13 wl1_13 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_14 bl0_0 br0_0 bl1_0 br1_0 wl0_14 wl1_14 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_15 bl0_0 br0_0 bl1_0 br1_0 wl0_15 wl1_15 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_16 bl0_0 br0_0 bl1_0 br1_0 wl0_16 wl1_16 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_17 bl0_0 br0_0 bl1_0 br1_0 wl0_17 wl1_17 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_18 bl0_0 br0_0 bl1_0 br1_0 wl0_18 wl1_18 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_19 bl0_0 br0_0 bl1_0 br1_0 wl0_19 wl1_19 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_20 bl0_0 br0_0 bl1_0 br1_0 wl0_20 wl1_20 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_21 bl0_0 br0_0 bl1_0 br1_0 wl0_21 wl1_21 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_22 bl0_0 br0_0 bl1_0 br1_0 wl0_22 wl1_22 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_23 bl0_0 br0_0 bl1_0 br1_0 wl0_23 wl1_23 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_24 bl0_0 br0_0 bl1_0 br1_0 wl0_24 wl1_24 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_25 bl0_0 br0_0 bl1_0 br1_0 wl0_25 wl1_25 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_26 bl0_0 br0_0 bl1_0 br1_0 wl0_26 wl1_26 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_27 bl0_0 br0_0 bl1_0 br1_0 wl0_27 wl1_27 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_28 bl0_0 br0_0 bl1_0 br1_0 wl0_28 wl1_28 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_29 bl0_0 br0_0 bl1_0 br1_0 wl0_29 wl1_29 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_30 bl0_0 br0_0 bl1_0 br1_0 wl0_30 wl1_30 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_31 bl0_0 br0_0 bl1_0 br1_0 wl0_31 wl1_31 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_32 bl0_0 br0_0 bl1_0 br1_0 wl0_32 wl1_32 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_33 bl0_0 br0_0 bl1_0 br1_0 wl0_33 wl1_33 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_34 bl0_0 br0_0 bl1_0 br1_0 wl0_34 wl1_34 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_35 bl0_0 br0_0 bl1_0 br1_0 wl0_35 wl1_35 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_36 bl0_0 br0_0 bl1_0 br1_0 wl0_36 wl1_36 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_37 bl0_0 br0_0 bl1_0 br1_0 wl0_37 wl1_37 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_38 bl0_0 br0_0 bl1_0 br1_0 wl0_38 wl1_38 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_39 bl0_0 br0_0 bl1_0 br1_0 wl0_39 wl1_39 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_40 bl0_0 br0_0 bl1_0 br1_0 wl0_40 wl1_40 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_41 bl0_0 br0_0 bl1_0 br1_0 wl0_41 wl1_41 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_42 bl0_0 br0_0 bl1_0 br1_0 wl0_42 wl1_42 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_43 bl0_0 br0_0 bl1_0 br1_0 wl0_43 wl1_43 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_44 bl0_0 br0_0 bl1_0 br1_0 wl0_44 wl1_44 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_45 bl0_0 br0_0 bl1_0 br1_0 wl0_45 wl1_45 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_46 bl0_0 br0_0 bl1_0 br1_0 wl0_46 wl1_46 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_47 bl0_0 br0_0 bl1_0 br1_0 wl0_47 wl1_47 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_48 bl0_0 br0_0 bl1_0 br1_0 wl0_48 wl1_48 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_49 bl0_0 br0_0 bl1_0 br1_0 wl0_49 wl1_49 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_50 bl0_0 br0_0 bl1_0 br1_0 wl0_50 wl1_50 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_51 bl0_0 br0_0 bl1_0 br1_0 wl0_51 wl1_51 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_52 bl0_0 br0_0 bl1_0 br1_0 wl0_52 wl1_52 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_53 bl0_0 br0_0 bl1_0 br1_0 wl0_53 wl1_53 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_54 bl0_0 br0_0 bl1_0 br1_0 wl0_54 wl1_54 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_55 bl0_0 br0_0 bl1_0 br1_0 wl0_55 wl1_55 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_56 bl0_0 br0_0 bl1_0 br1_0 wl0_56 wl1_56 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_57 bl0_0 br0_0 bl1_0 br1_0 wl0_57 wl1_57 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_58 bl0_0 br0_0 bl1_0 br1_0 wl0_58 wl1_58 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_59 bl0_0 br0_0 bl1_0 br1_0 wl0_59 wl1_59 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_60 bl0_0 br0_0 bl1_0 br1_0 wl0_60 wl1_60 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_61 bl0_0 br0_0 bl1_0 br1_0 wl0_61 wl1_61 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_62 bl0_0 br0_0 bl1_0 br1_0 wl0_62 wl1_62 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_63 bl0_0 br0_0 bl1_0 br1_0 wl0_63 wl1_63 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_64 bl0_0 br0_0 bl1_0 br1_0 wl0_64 wl1_64 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_65 bl0_0 br0_0 bl1_0 br1_0 wl0_65 wl1_65 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_66 bl0_0 br0_0 bl1_0 br1_0 wl0_66 wl1_66 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_67 bl0_0 br0_0 bl1_0 br1_0 wl0_67 wl1_67 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_68 bl0_0 br0_0 bl1_0 br1_0 wl0_68 wl1_68 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_69 bl0_0 br0_0 bl1_0 br1_0 wl0_69 wl1_69 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_70 bl0_0 br0_0 bl1_0 br1_0 wl0_70 wl1_70 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_71 bl0_0 br0_0 bl1_0 br1_0 wl0_71 wl1_71 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_72 bl0_0 br0_0 bl1_0 br1_0 wl0_72 wl1_72 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_73 bl0_0 br0_0 bl1_0 br1_0 wl0_73 wl1_73 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_74 bl0_0 br0_0 bl1_0 br1_0 wl0_74 wl1_74 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_75 bl0_0 br0_0 bl1_0 br1_0 wl0_75 wl1_75 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_76 bl0_0 br0_0 bl1_0 br1_0 wl0_76 wl1_76 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_77 bl0_0 br0_0 bl1_0 br1_0 wl0_77 wl1_77 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_78 bl0_0 br0_0 bl1_0 br1_0 wl0_78 wl1_78 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_79 bl0_0 br0_0 bl1_0 br1_0 wl0_79 wl1_79 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_80 bl0_0 br0_0 bl1_0 br1_0 wl0_80 wl1_80 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_81 bl0_0 br0_0 bl1_0 br1_0 wl0_81 wl1_81 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_82 bl0_0 br0_0 bl1_0 br1_0 wl0_82 wl1_82 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_83 bl0_0 br0_0 bl1_0 br1_0 wl0_83 wl1_83 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_84 bl0_0 br0_0 bl1_0 br1_0 wl0_84 wl1_84 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_85 bl0_0 br0_0 bl1_0 br1_0 wl0_85 wl1_85 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_86 bl0_0 br0_0 bl1_0 br1_0 wl0_86 wl1_86 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_87 bl0_0 br0_0 bl1_0 br1_0 wl0_87 wl1_87 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_88 bl0_0 br0_0 bl1_0 br1_0 wl0_88 wl1_88 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_89 bl0_0 br0_0 bl1_0 br1_0 wl0_89 wl1_89 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_90 bl0_0 br0_0 bl1_0 br1_0 wl0_90 wl1_90 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_91 bl0_0 br0_0 bl1_0 br1_0 wl0_91 wl1_91 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_92 bl0_0 br0_0 bl1_0 br1_0 wl0_92 wl1_92 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_93 bl0_0 br0_0 bl1_0 br1_0 wl0_93 wl1_93 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_94 bl0_0 br0_0 bl1_0 br1_0 wl0_94 wl1_94 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_95 bl0_0 br0_0 bl1_0 br1_0 wl0_95 wl1_95 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_96 bl0_0 br0_0 bl1_0 br1_0 wl0_96 wl1_96 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_97 bl0_0 br0_0 bl1_0 br1_0 wl0_97 wl1_97 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_98 bl0_0 br0_0 bl1_0 br1_0 wl0_98 wl1_98 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_99 bl0_0 br0_0 bl1_0 br1_0 wl0_99 wl1_99 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_100 bl0_0 br0_0 bl1_0 br1_0 wl0_100 wl1_100 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_101 bl0_0 br0_0 bl1_0 br1_0 wl0_101 wl1_101 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_102 bl0_0 br0_0 bl1_0 br1_0 wl0_102 wl1_102 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_103 bl0_0 br0_0 bl1_0 br1_0 wl0_103 wl1_103 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_104 bl0_0 br0_0 bl1_0 br1_0 wl0_104 wl1_104 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_105 bl0_0 br0_0 bl1_0 br1_0 wl0_105 wl1_105 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_106 bl0_0 br0_0 bl1_0 br1_0 wl0_106 wl1_106 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_107 bl0_0 br0_0 bl1_0 br1_0 wl0_107 wl1_107 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_108 bl0_0 br0_0 bl1_0 br1_0 wl0_108 wl1_108 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_109 bl0_0 br0_0 bl1_0 br1_0 wl0_109 wl1_109 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_110 bl0_0 br0_0 bl1_0 br1_0 wl0_110 wl1_110 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_111 bl0_0 br0_0 bl1_0 br1_0 wl0_111 wl1_111 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_112 bl0_0 br0_0 bl1_0 br1_0 wl0_112 wl1_112 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_113 bl0_0 br0_0 bl1_0 br1_0 wl0_113 wl1_113 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_114 bl0_0 br0_0 bl1_0 br1_0 wl0_114 wl1_114 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_115 bl0_0 br0_0 bl1_0 br1_0 wl0_115 wl1_115 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_116 bl0_0 br0_0 bl1_0 br1_0 wl0_116 wl1_116 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_117 bl0_0 br0_0 bl1_0 br1_0 wl0_117 wl1_117 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_118 bl0_0 br0_0 bl1_0 br1_0 wl0_118 wl1_118 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_119 bl0_0 br0_0 bl1_0 br1_0 wl0_119 wl1_119 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_120 bl0_0 br0_0 bl1_0 br1_0 wl0_120 wl1_120 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_121 bl0_0 br0_0 bl1_0 br1_0 wl0_121 wl1_121 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_122 bl0_0 br0_0 bl1_0 br1_0 wl0_122 wl1_122 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_123 bl0_0 br0_0 bl1_0 br1_0 wl0_123 wl1_123 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_124 bl0_0 br0_0 bl1_0 br1_0 wl0_124 wl1_124 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_125 bl0_0 br0_0 bl1_0 br1_0 wl0_125 wl1_125 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_126 bl0_0 br0_0 bl1_0 br1_0 wl0_126 wl1_126 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_127 bl0_0 br0_0 bl1_0 br1_0 wl0_127 wl1_127 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_128 bl0_0 br0_0 bl1_0 br1_0 wl0_128 wl1_128 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_129 bl0_0 br0_0 bl1_0 br1_0 wl0_129 wl1_129 vdd gnd sky130_fd_bd_sram__openram_dp_cell_replica
Xrbc_130 bl0_0 br0_0 bl1_0 br1_0 wl0_130 wl1_130 vdd gnd sky130_fd_bd_sram__openram_dp_cell_dummy
.ENDS replica_column
.SUBCKT sky130_fd_bd_sram__openram_dp_cell_replica bl0 br0 bl1 br1 wl0 wl1 vdd gnd
** N=9 EP=8 IP=0 FDC=16
*.SEEDPROM

* Bitcell Core
X0 Q wl1 bl1 gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1
X1 gnd vdd Q gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1
X2 gnd vdd Q gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1
X3 bl0 wl0 Q gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1
X4 vdd wl1 br1 gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1
X5 gnd Q vdd gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1
X6 gnd Q vdd gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1
X7 br0 wl0 vdd gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1
X8 vdd Q vdd vdd sky130_fd_pr__special_pfet_latch W=0.14 L=0.15 m=1 mult=1
X9 Q vdd vdd vdd sky130_fd_pr__special_pfet_latch W=0.14 L=0.15 m=1 mult=1

* drainOnly PMOS
X10 vdd wl1 vdd vdd sky130_fd_pr__special_pfet_pass L=0.08 W=0.14 m=1 mult=1
X11 Q wl0 Q vdd sky130_fd_pr__special_pfet_pass L=0.08 W=0.14 m=1 mult=1

* drainOnly NMOS
X12 bl1 gnd bl1 gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.08 m=1 mult=1
X14 br1 gnd br1 gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.08 m=1 mult=1

.ENDS
.SUBCKT sky130_fd_bd_sram__openram_dp_cell_dummy bl0 br0 bl1 br1 wl0 wl1 vdd gnd
** N=14 EP=6 IP=0 FDC=16
*.SEEDPROM

* Bitcell Core
X1 1 gnd gnd gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1
X2 1 wl1 bl1 gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1
X3 2 gnd gnd gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1
X4 2 wl1 br1 gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1
X5 3 gnd gnd gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1
X6 3 wl0 bl0 gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1
X7 4 gnd gnd gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1
X8 4 wl0 br0 gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.15 m=1 mult=1

* drainOnly NMOS
X9 bl1 gnd bl1 gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.08 m=1 mult=1
X10 br1 gnd br1 gnd sky130_fd_pr__special_nfet_latch W=0.21 L=0.08 m=1 mult=1

.ENDS
