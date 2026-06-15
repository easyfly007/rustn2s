* Double-Tail Dynamic Comparator (SKY130 PDK) [n2s test case 30]
*
* === n2s test-case metadata ===
* Source:       myadc repo, sky130/comparator/double_tail_comp.spice
*               (same comparator as case 26, ported to SKY130 130nm, VDD=1.8V)
* Circuit type: Double-tail dynamic comparator, SKY130 PDK device modeling
* Devices:      24 total -- 16 transistors + 4 capacitor + 4 voltage source
* MOSFETs:      16, modeled as sky130_fd_pr__*fet X instances (9 nfet, 7 pfet)
* Notes:        PDK style -- transistors are subckt instances with NO in-file
*               .subckt definition; tests X-without-local-def rendering.
* Added:        2026-06-15  (real-world test-set enrichment)
* ===============================
* Double-Tail Dynamic Comparator for 8-bit SAR ADC
* SKY130 130nm, VDD=1.8V
* Target: resolve 3.5mV within 5ns
*
* Run: ngspice -b sky130/comparator/double_tail_comp.spice

.lib "/home/yifei/sky130A/libs.tech/ngspice/sky130.lib.spice" tt

.param VDD=1.8
.param VCM=0.9
.param VDIFF=3.5m

* Power supply
VVDD vdd 0 DC {VDD}

* Input signals: VCM +/- VDIFF/2
VINP inp 0 DC 'VCM + VDIFF/2'
VINN inn 0 DC 'VCM - VDIFF/2'

* Clock: period=100ns (10MHz internal SAR clock @1MS/s)
* Rise/fall = 50ps, pulse width = 45ns, period = 100ns
VCLK clk 0 PULSE(0 {VDD} 10n 50p 50p 45n 100n)

*** First Stage (Pre-amplifier) ***

* Tail current NMOS (clocked): gate=clk, source=GND
XN_TAIL ntail clk 0 0 sky130_fd_pr__nfet_01v8 W=5.0 L=0.15

* Input diff pair
XN1 fn inn ntail 0 sky130_fd_pr__nfet_01v8 W=3.0 L=0.15
XN2 fp inp ntail 0 sky130_fd_pr__nfet_01v8 W=3.0 L=0.15

* PMOS loads (clocked, reset to VDD when CLK=0)
XP1 fn clk vdd vdd sky130_fd_pr__pfet_01v8 W=2.0 L=0.15
XP2 fp clk vdd vdd sky130_fd_pr__pfet_01v8 W=2.0 L=0.15

*** Second Stage (Latch) ***

* PMOS cross-coupled pair
XP3 outn outp vdd vdd sky130_fd_pr__pfet_01v8 W=3.0 L=0.15
XP4 outp outn vdd vdd sky130_fd_pr__pfet_01v8 W=3.0 L=0.15

* NMOS cross-coupled pair (with clocked tail to avoid reset contention)
XN3 outn outp sn 0 sky130_fd_pr__nfet_01v8 W=3.0 L=0.15
XN4 outp outn sn 0 sky130_fd_pr__nfet_01v8 W=3.0 L=0.15
XN_TAIL2 sn clk 0 0 sky130_fd_pr__nfet_01v8 W=5.0 L=0.15

* Second-stage input (Schinkel topology): fn/fp drive outn/outp directly
XN5 outn fn clkb 0 sky130_fd_pr__nfet_01v8 W=3.0 L=0.15
XN6 outp fp clkb 0 sky130_fd_pr__nfet_01v8 W=3.0 L=0.15

* Reset PMOS for latch outputs (reset to VDD when CLK=0)
XP5 outn clk vdd vdd sky130_fd_pr__pfet_01v8 W=1.0 L=0.15
XP6 outp clk vdd vdd sky130_fd_pr__pfet_01v8 W=1.0 L=0.15

* Clock inverter for clkb
XP_INV clkb clk vdd vdd sky130_fd_pr__pfet_01v8 W=1.0 L=0.15
XN_INV clkb clk 0 0 sky130_fd_pr__nfet_01v8 W=0.42 L=0.15

*** Parasitic load capacitance ***
CL1 outn 0 10f
CL2 outp 0 10f
CF1 fn 0 5f
CF2 fp 0 5f

*** Simulation ***
.control
* Step 1: Large signal (50mV) functional check
tran 0.1n 500n

echo ""
echo "=== Double-Tail Comparator - SKY130 1.8V ==="
echo ""

* Measure decision time in 2nd clock cycle
* clk rises at 110ns, measure when outn or outp resolves
meas tran t_clk_rise WHEN v(clk)=0.9 RISE=2
meas tran t_outn_fall WHEN v(outn)=0.9 FALL=2
meas tran t_outp_fall WHEN v(outp)=0.9 FALL=2

echo ""
echo "--- Decision time measurement (2nd cycle) ---"
echo "clk rise:"
print t_clk_rise
echo "outn fall:"
print t_outn_fall
echo "outp fall:"
print t_outp_fall

* Decision time = whichever output falls first after clk rise
* For inp > inn: outn should fall (outn→0), outp stays high
* So decision time = t_outn_fall - t_clk_rise

echo ""
echo "--- Waveform check ---"
echo "inp > inn, so expect: outn → 0, outp → VDD"

* Check final values in 2nd evaluation phase (at t = 150ns)
meas tran v_outn_eval FIND v(outn) AT=150n
meas tran v_outp_eval FIND v(outp) AT=150n
meas tran v_fn_eval FIND v(fn) AT=150n
meas tran v_fp_eval FIND v(fp) AT=150n

echo "outn at 150ns:"
print v_outn_eval
echo "outp at 150ns:"
print v_outp_eval
echo "fn at 150ns:"
print v_fn_eval
echo "fp at 150ns:"
print v_fp_eval

wrdata sky130/comparator/comp_tran v(clk) v(outp) v(outn) v(fn) v(fp)

echo ""
echo "=== Comparator simulation complete ==="

.endc

.end
