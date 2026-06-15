* Double-Tail Dynamic Comparator (PTM 28nm) [n2s test case 26]
*
* === n2s test-case metadata ===
* Source:       myadc repo, ptm28nm/comparator/double_tail_comp.spice
*               (real 8-bit SAR-ADC design, PTM 28nm CMOS, VDD=0.9V)
* Circuit type: Double-tail dynamic comparator
*               (clocked pre-amplifier + cross-coupled regenerative latch)
* Devices:      23 total -- 15 MOSFET, 4 capacitor, 4 voltage source
* MOSFETs:      15  (8 NMOS, 7 PMOS)
* Notes:        flat netlist; exercises cross-coupled pairs + clocked tails.
* Added:        2026-06-15  (real-world test-set enrichment)
* ===============================
* Double-Tail Dynamic Comparator for 8-bit SAR ADC
* 28nm CMOS, VDD=0.9V
* Target: resolve 0.9V/256 = 3.5mV within ~500ps

.include ../models/nmos_28nm.mod
.include ../models/pmos_28nm.mod

.param VDD=0.9
.param VCM=0.45
.param VDIFF=3.5m

* Power supply
VVDD vdd 0 DC VDD

* Input signals: VCM +/- VDIFF/2
VINP inp 0 DC 'VCM + VDIFF/2'
VINN inn 0 DC 'VCM - VDIFF/2'

* Clock: period=1ns (1GHz internal SAR clock)
* Rise/fall = 30ps, pulse width = 450ps, period = 1ns
VCLK clk 0 PULSE(0 VDD 0 30p 30p 450p 1n)

*** First Stage (Pre-amplifier) ***
* Tail current NMOS (clocked)
MN_TAIL ntail 0 clk 0 nch L=28n W=4u

* Input diff pair
MN1 fn inn ntail 0 nch L=28n W=2u
MN2 fp inp ntail 0 nch L=28n W=2u

* PMOS loads (clocked, reset to VDD when CLK=0)
MP1 fn clk vdd vdd pch L=28n W=1u
MP2 fp clk vdd vdd pch L=28n W=1u

*** Second Stage (Latch) ***
* PMOS cross-coupled pair
MP3 outn outp vdd vdd pch L=28n W=2u
MP4 outp outn vdd vdd pch L=28n W=2u

* NMOS cross-coupled pair
MN3 outn outp sn 0 nch L=28n W=2u
MN4 outp outn sn 0 nch L=28n W=2u

* NMOS tail for latch (driven by fn/fp from first stage)
MN5 sn fn clkb 0 nch L=28n W=2u
MN6 sn fp clkb 0 nch L=28n W=2u

* Reset PMOS for latch outputs (reset to VDD when CLK=0)
MP5 outn clk vdd vdd pch L=28n W=0.5u
MP6 outp clk vdd vdd pch L=28n W=0.5u

* Clock inverter for clkb
MP_INV clkb clk vdd vdd pch L=28n W=0.5u
MN_INV clkb clk 0 0 nch L=28n W=0.25u

*** Parasitic load capacitance ***
CL1 outn 0 5f
CL2 outp 0 5f
CF1 fn 0 2f
CF2 fp 0 2f

*** Simulation ***
.tran 1p 5n

.control
run
wrdata comp_tran v(clk) v(outp) v(outn) v(fn) v(fp)
echo "Comparator transient simulation done."
.endc

.end
