* Asynchronous 8-bit SAR Control Logic (PTM 28nm) [n2s test case 29]
*
* === n2s test-case metadata ===
* Source:       myadc repo, ptm28nm/sar_logic/async_sar.spice
*               (full 8-bit asynchronous SAR digital controller)
* Circuit type: Asynchronous SAR control logic (digital, hierarchical)
* Devices:      92 total -- 88 gate/latch subckt instances + 2 cap + 2 source.
*               Instances: 37 inv, 13 nand2, 8 srlatch_r, 8 bitreg,
*               8 blank_delay, 8 prop_delay, 6 nor2.
* MOSFETs:      0 at top level -- gates are X instances whose .subckt defs
*               live in the .include'd gates.spice (not resolved by n2s).
* Notes:        SCALE stress -- largest case in the suite (92 components).
*               Tests X instances with externally-defined subckts; a few
*               unsupported S/W switch lines are skipped.
* Added:        2026-06-15  (real-world test-set enrichment)
* ===============================
* Asynchronous SAR Logic - 8-bit
* Uses comparator valid signal to trigger next bit conversion
* sample=1: reset all latches; sample falling edge: start conversion

.include gates.spice

*** Power ***
VVDD vdd 0 DC 0.9

*** Sample clock ***
* High=sample/reset, Low=conversion
VCLK_S sample 0 PULSE(0.9 0 2n 50p 50p 5n 20n)

*** Resettable SR Latch ***
.subckt srlatch_r sb rb q qb vdd vss rst
Xinv_rst rst rst_b vdd vss inv
* sb_eff = OR(sb, rst): when rst=1, sb_eff=1 (block set)
XNOR_SB sb rst sb_nor vdd vss nor2
Xinv_sbeff sb_nor sb_eff vdd vss inv
* rb_eff = AND(rb, rst_b): when rst=1, rb_eff=0 (force reset)
XNAND_RB rb rst_b rb_nand vdd vss nand2
Xinv_rb rb_nand rb_eff vdd vss inv
* Core NAND SR latch
XNAND1 sb_eff qb q vdd vss nand2
XNAND2 rb_eff q qb vdd vss nand2
.ends srlatch_r

*** Valid Detector ***
XVALID outp outn valid_raw vdd 0 nand2
XVAL_B1 valid_raw valid_b vdd 0 inv
XVAL_B2 valid_b valid vdd 0 inv

*** Start pulse: on falling edge of sample ***
Xinv_samp sample sample_b vdd 0 inv
Xsd1 sample_b sd1 vdd 0 inv
Xsd2 sd1 sd2 vdd 0 inv
Xsd3 sd2 sd3 vdd 0 inv
Xsd4 sd3 sd4 vdd 0 inv
Xsd5 sd4 sd5 vdd 0 inv
Xsd6 sd5 sd6 vdd 0 inv
Xinv_sd6 sd6 sd6_b vdd 0 inv
XSTART_NAND sample_b sd6_b start_b vdd 0 nand2

*** Phase shift register (one-hot, MSB first) ***
*
* Architecture: Each phase uses a "blanking delay" on its output.
* The clear NAND uses ph_stable (= ph delayed by ~200ps) instead of ph.
* This prevents the stale valid signal from immediately clearing a newly
* activated phase. By the time ph_stable goes HIGH, the comparator has
* reset and valid has gone LOW, so only a fresh comparison can clear it.
*
* Trigger chain: clr_b -> 2-inv prop_delay -> trig_raw -> INV -> set_b
*   Short delay (~25ps) since blanking prevents premature clear.

*** Blanking delay subcircuit (16 inverters, ~180ps rising edge delay) ***
* Uses default inverter sizing (wp=0.5u, wn=0.25u)
.subckt blank_delay in out vdd vss
Xb1  in  n1  vdd vss inv
Xb2  n1  n2  vdd vss inv
Xb3  n2  n3  vdd vss inv
Xb4  n3  n4  vdd vss inv
Xb5  n4  n5  vdd vss inv
Xb6  n5  n6  vdd vss inv
Xb7  n6  n7  vdd vss inv
Xb8  n7  n8  vdd vss inv
Xb9  n8  n9  vdd vss inv
Xb10 n9  n10 vdd vss inv
Xb11 n10 n11 vdd vss inv
Xb12 n11 n12 vdd vss inv
Xb13 n12 n13 vdd vss inv
Xb14 n13 n14 vdd vss inv
Xb15 n14 n15 vdd vss inv
Xb16 n15 out vdd vss inv
.ends blank_delay

*** Propagation delay subcircuit (2 inverters, ~25ps) ***
.subckt prop_delay in out vdd vss
Xd1 in n1 vdd vss inv
Xd2 n1 out vdd vss inv
.ends prop_delay

*** Phase 7 (MSB, first) ***
Xblank7 ph7 ph7_stable vdd 0 blank_delay
XCLR7_AND valid ph7_stable clr7_pre vdd 0 nand2
Xinv_clr7 clr7_pre clr7_b vdd 0 inv
XSR7 start_b clr7_pre ph7 ph7_b vdd 0 sample srlatch_r
* Trigger chain to phase 6
Xdly7 clr7_b trig6_raw vdd 0 prop_delay
Xinv_s6 trig6_raw set6_b vdd 0 inv

*** Phase 6 ***
Xblank6 ph6 ph6_stable vdd 0 blank_delay
XCLR6_AND valid ph6_stable clr6_pre vdd 0 nand2
Xinv_clr6 clr6_pre clr6_b vdd 0 inv
XSR6 set6_b clr6_pre ph6 ph6_b vdd 0 sample srlatch_r
Xdly6 clr6_b trig5_raw vdd 0 prop_delay
Xinv_s5 trig5_raw set5_b vdd 0 inv

*** Phase 5 ***
Xblank5 ph5 ph5_stable vdd 0 blank_delay
XCLR5_AND valid ph5_stable clr5_pre vdd 0 nand2
Xinv_clr5 clr5_pre clr5_b vdd 0 inv
XSR5 set5_b clr5_pre ph5 ph5_b vdd 0 sample srlatch_r
Xdly5 clr5_b trig4_raw vdd 0 prop_delay
Xinv_s4 trig4_raw set4_b vdd 0 inv

*** Phase 4 ***
Xblank4 ph4 ph4_stable vdd 0 blank_delay
XCLR4_AND valid ph4_stable clr4_pre vdd 0 nand2
Xinv_clr4 clr4_pre clr4_b vdd 0 inv
XSR4 set4_b clr4_pre ph4 ph4_b vdd 0 sample srlatch_r
Xdly4 clr4_b trig3_raw vdd 0 prop_delay
Xinv_s3 trig3_raw set3_b vdd 0 inv

*** Phase 3 ***
Xblank3 ph3 ph3_stable vdd 0 blank_delay
XCLR3_AND valid ph3_stable clr3_pre vdd 0 nand2
Xinv_clr3 clr3_pre clr3_b vdd 0 inv
XSR3 set3_b clr3_pre ph3 ph3_b vdd 0 sample srlatch_r
Xdly3 clr3_b trig2_raw vdd 0 prop_delay
Xinv_s2 trig2_raw set2_b vdd 0 inv

*** Phase 2 ***
Xblank2 ph2 ph2_stable vdd 0 blank_delay
XCLR2_AND valid ph2_stable clr2_pre vdd 0 nand2
Xinv_clr2 clr2_pre clr2_b vdd 0 inv
XSR2 set2_b clr2_pre ph2 ph2_b vdd 0 sample srlatch_r
Xdly2 clr2_b trig1_raw vdd 0 prop_delay
Xinv_s1 trig1_raw set1_b vdd 0 inv

*** Phase 1 ***
Xblank1 ph1 ph1_stable vdd 0 blank_delay
XCLR1_AND valid ph1_stable clr1_pre vdd 0 nand2
Xinv_clr1 clr1_pre clr1_b vdd 0 inv
XSR1 set1_b clr1_pre ph1 ph1_b vdd 0 sample srlatch_r
Xdly1 clr1_b trig0_raw vdd 0 prop_delay
Xinv_s0 trig0_raw set0_b vdd 0 inv

*** Phase 0 (LSB, last) ***
Xblank0 ph0 ph0_stable vdd 0 blank_delay
XCLR0_AND valid ph0_stable clr0_pre vdd 0 nand2
Xinv_clr0 clr0_pre clr0_b vdd 0 inv
XSR0 set0_b clr0_pre ph0 ph0_b vdd 0 sample srlatch_r
* Done signal: buffer (2 INV) preserves polarity: idle=0, event=1
Xdly0 clr0_b done_raw vdd 0 prop_delay
Xinv_d1 done_raw done_b vdd 0 inv
Xinv_d2 done_b done vdd 0 inv

*** Comparator Clock = any phase active ***
XNOR01 ph0 ph1 n01 vdd 0 nor2
XNOR23 ph2 ph3 n23 vdd 0 nor2
XNOR45 ph4 ph5 n45 vdd 0 nor2
XNOR67 ph6 ph7 n67 vdd 0 nor2
XNAND_A n01 n23 na vdd 0 nand2
XNAND_B n45 n67 nb vdd 0 nand2
XNOR_AB na nb comp_clk_pre vdd 0 nor2
Xinv_cc1 comp_clk_pre comp_clk vdd 0 inv

*** Bit Registers ***
* Bit is set (q=1) by set_b going LOW (optimistic set)
* Bit is cleared (q=0) only when valid=1 AND outn=1 AND phase=1
* (comparator says negative result during this phase)
.subckt bitreg set_b valid outn phase q qb vdd vss rst
XCLR valid outn phase clr_b vdd vss nand3
XSR set_b clr_b q qb vdd vss rst srlatch_r
.ends bitreg

XBR7 start_b valid outn ph7 bit7 bit7_b vdd 0 sample bitreg
XBR6 set6_b valid outn ph6 bit6 bit6_b vdd 0 sample bitreg
XBR5 set5_b valid outn ph5 bit5 bit5_b vdd 0 sample bitreg
XBR4 set4_b valid outn ph4 bit4 bit4_b vdd 0 sample bitreg
XBR3 set3_b valid outn ph3 bit3 bit3_b vdd 0 sample bitreg
XBR2 set2_b valid outn ph2 bit2 bit2_b vdd 0 sample bitreg
XBR1 set1_b valid outn ph1 bit1 bit1_b vdd 0 sample bitreg
XBR0 set0_b valid outn ph0 bit0 bit0_b vdd 0 sample bitreg

*** Simulated Comparator (behavioral) ***
* rst_comp = NOT(comp_clk) OR sample = 1 when NOT comparing
Xinv_cc comp_clk comp_clk_n vdd 0 inv
XRST_OR comp_clk_n sample rst_comp_nor vdd 0 nor2
Xinv_rst_c rst_comp_nor rst_comp vdd 0 inv

.model sw_rst sw vt=0.45 vh=0.05 ron=10 roff=1e12
.model sw_res sw vt=0.45 vh=0.05 ron=10 roff=1e12

* Reset: pull outp, outn -> VDD when rst_comp=1
S_prst outp vdd rst_comp 0 sw_rst
S_nrst outn vdd rst_comp 0 sw_rst

* Resolve delay: digital delay chain (6 inverters, ~70ps)
* Resets cleanly unlike RC circuit
Xinv_rc1 rst_comp rd1 vdd 0 inv
Xinv_rc2 rd1 rd2 vdd 0 inv
Xinv_rc3 rd2 rd3 vdd 0 inv
Xinv_rc4 rd3 rd4 vdd 0 inv
Xinv_rc5 rd4 rd5 vdd 0 inv
Xinv_rc6 rd5 rc_trig vdd 0 inv
* rc_trig: follows rst_comp inverted (= comparing mode), delayed ~70ps
* rc_trig=0 during reset, rc_trig=1 after ~70ps of comparing

* resolve_en = AND(rc_trig, comparing_mode)
XNAND_res rc_trig rst_comp_nor res_nand vdd 0 nand2
Xinv_res res_nand resolve_en vdd 0 inv

* Pull outn low when resolved (simulate positive result: all bits stay 1)
S_nres outn 0 resolve_en 0 sw_res

* Load caps
C_outp outp 0 5f
C_outn outn 0 5f

*** Simulation ***
.tran 1p 25n

.control
run
wrdata sar_logic_out v(sample) v(comp_clk) v(valid) v(ph7) v(ph6) v(ph5) v(ph4) v(ph3) v(ph2) v(ph1) v(ph0) v(bit7) v(bit6) v(bit5) v(bit4) v(bit3) v(bit2) v(bit1) v(bit0) v(done) v(outp) v(outn)
echo "SAR logic simulation done."
.endc

.end
