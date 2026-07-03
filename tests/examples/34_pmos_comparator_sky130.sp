* PMOS-Input Double-Tail Dynamic Comparator (SKY130) [n2s test case 34]
*
* === n2s test-case metadata ===
* Source:       myadc repo, sky130/comparator/pmos_double_tail_comp.spice
* Circuit type: PMOS-input double-tail dynamic comparator (mirror image of
*               the NMOS-input cases 26/30)
* Devices:      24 total -- 16 transistors + 4 capacitor + 4 voltage source
* MOSFETs:      16 as sky130_fd_pr__*fet X instances (7 nfet, 9 pfet)
* Notes:        PMOS diff pair on TOP rail (input pair sources from vdd via
*               ptail) -- exercises polarity sorting where the signal flows
*               PMOS -> NMOS, opposite of the textbook NMOS-input layout.
*               Includes .control block with foreach loop (parser must skip).
* Added:        2026-07-03  (real-world test-set enrichment, batch 2)
* ===============================
* PMOS-Input Double-Tail Dynamic Comparator for 8-bit SAR ADC
* SKY130 130nm, VDD=1.8V
*
* Run: ngspice -b sky130/comparator/pmos_double_tail_comp.spice

.lib "/home/yifei/sky130A/libs.tech/ngspice/sky130.lib.spice" tt

.param VDD=1.8
.param VTOP=0.05

VVDD vdd 0 DC {VDD}
VINP inp 0 DC {VTOP}
VINN inn 0 DC 0
VCLK clk 0 PULSE(0 {VDD} 10n 50p 50p 45n 100n)

*** Clock inverter ***
XPC clkb clk vdd vdd sky130_fd_pr__pfet_01v8 W=1.0 L=0.15
XNC clkb clk 0 0 sky130_fd_pr__nfet_01v8 W=0.42 L=0.15

*** Stage 1: PMOS Pre-amplifier ***
XPT ptail clkb vdd vdd sky130_fd_pr__pfet_01v8 W=5.0 L=0.15
XP1 fn inn ptail vdd sky130_fd_pr__pfet_01v8 W=3.0 L=0.15
XP2 fp inp ptail vdd sky130_fd_pr__pfet_01v8 W=3.0 L=0.15
XNR1 fn clkb 0 0 sky130_fd_pr__nfet_01v8 W=2.0 L=0.15
XNR2 fp clkb 0 0 sky130_fd_pr__nfet_01v8 W=2.0 L=0.15

*** Stage 2: Latch (same topology as NMOS version) ***
* PMOS cross-coupled (source=VDD)
XP3 outn outp vdd vdd sky130_fd_pr__pfet_01v8 W=3.0 L=0.15
XP4 outp outn vdd vdd sky130_fd_pr__pfet_01v8 W=3.0 L=0.15

* NMOS cross-coupled + tail
XN3 outn outp sn 0 sky130_fd_pr__nfet_01v8 W=3.0 L=0.15
XN4 outp outn sn 0 sky130_fd_pr__nfet_01v8 W=3.0 L=0.15
XNT2 sn clk 0 0 sky130_fd_pr__nfet_01v8 W=5.0 L=0.15

* NMOS injection with WEAK transistors to avoid overwhelming latch
* fn/fp rise to VDD → Vgs=VDD → very strong if W is large
* Use minimum width to provide differential seed without overwhelming
XN5 outn fn 0 0 sky130_fd_pr__nfet_01v8 W=0.42 L=0.15
XN6 outp fp 0 0 sky130_fd_pr__nfet_01v8 W=0.42 L=0.15

* PMOS reset: pull to VDD when clk=0
XPR1 outn clk vdd vdd sky130_fd_pr__pfet_01v8 W=1.0 L=0.15
XPR2 outp clk vdd vdd sky130_fd_pr__pfet_01v8 W=1.0 L=0.15

*** Load capacitance ***
CL1 outn 0 10f
CL2 outp 0 10f
CF1 fn 0 5f
CF2 fp 0 5f

*** Simulation ***
.control
echo "=== PMOS comp v3: weak NMOS injection ==="
echo "vtop(V)  outn(V)  outp(V)"

foreach vt 0.8 0.5 0.2 0.1 0.05 0.01 0.005 -0.005 -0.01 -0.05 -0.1 -0.2 -0.5 -0.8
    alterparam VTOP = $vt
    reset
    tran 0.1n 300n
    meas tran v_on FIND v(outn) AT=150n
    meas tran v_op FIND v(outp) AT=150n
    echo "$vt  $&v_on  $&v_op"
end

echo "Expected: vtop>0 → outn=0, outp=VDD | vtop<0 → outn=VDD, outp=0"
.endc
.end
