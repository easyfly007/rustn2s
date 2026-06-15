* Bootstrapped Sampling Switch (PTM 28nm) [n2s test case 27]
*
* === n2s test-case metadata ===
* Source:       myadc repo, ptm28nm/bootstrap/bootstrap_sw.spice
*               (sample-and-hold front-end of a SAR ADC, PTM 28nm)
* Circuit type: Bootstrapped (switched-capacitor) sampling switch -- HIERARCHICAL
* Devices:      5 at top level -- 1 subckt box, 1 sample cap, 3 sources.
*               The bootstrap_sw subckt holds 8 MOSFET + 2 capacitor + 1 R.
* MOSFETs:      8  (inside the subckt; rendered as a box in hierarchical mode)
* Notes:        tests X-instance + local .subckt hierarchical rendering.
* Added:        2026-06-15  (real-world test-set enrichment)
* ===============================
* Bootstrap Sampling Switch for 8-bit SAR ADC
* 28nm CMOS, VDD=0.9V
*
* Principle: Main switch M1 Vgs = VDD (constant, independent of Vin)
*   - Hold phase: Cboot charged to VDD (nt=VDD, nb=GND)
*   - Sample phase: Cboot bottom connects to M1 source,
*     top connects to M1 gate -> Vgate = Vin + VDD -> Vgs = VDD
*
* Topology: Abo-Gray style bootstrap (8 transistors + 1 cap)
*   M1: main sampling NMOS (triple-well, body=source)
*   M2: charge Cboot bottom to GND (hold)
*   M3: charge Cboot top to VDD (hold)
*   M4: pull M1 gate to GND (hold)
*   M5: connect Cboot bottom to M1 source (sample, gate=m5g -> bootstrapped)
*   M6: connect Cboot top to M1 gate (sample)
*   M7: pull M5 gate to GND during hold (prevents M5 leakage path)
*   M8: connect M5 gate to nt during sample (bootstrapped drive)

.include ../models/nmos_28nm.mod
.include ../models/pmos_28nm.mod
.include ../sar_logic/gates.spice

***** Bootstrap Switch Subcircuit *****
* Ports: in out phi vdd vss
.subckt bootstrap_sw in out phi vdd vss

* Clock inverter
Xinv1 phi phib vdd vss inv

* Main sampling NMOS
* Triple-well: body=source (out) to eliminate body effect
M1 in gate out out nch L=28n W=4u

* Bootstrap capacitor (charged to VDD during hold)
Cboot nb nt 500f

*** HOLD phase (phi=0, phib=1): charge Cboot to VDD ***

* M2: pull nb (Cboot bottom) to GND
M2 nb phib vss vss nch L=28n W=1u

* M3: pull nt (Cboot top) to VDD
M3 nt phi vdd vdd pch L=28n W=1u

* M4: pull M1 gate to GND -> M1 off
M4 gate phib vss vss nch L=28n W=1u

*** SAMPLE phase (phi=1, phib=0): bootstrap M1 ***

* M7: pull M5 gate (m5g) to GND during hold -> M5 off, no leakage
M7 m5g phib vss vss nch L=28n W=1u

* M8: connect M5 gate (m5g) to nt during sample (bootstrapped drive)
* PMOS: body=nt (nt can reach Vin+VDD > VDD, avoids forward-biased junction)
M8 m5g phib nt nt pch L=28n W=1u

* M5: connect nb to out (M1 source)
* Gate driven by m5g: during sample m5g=nt=Vin+VDD, Vgs_M5=VDD (fully on)
* During hold: m5g=0, M5 off -> no leakage path from Csamp
M5 nb m5g out vss nch L=28n W=1u

* M6: connect nt to M1 gate
* PMOS in dedicated n-well (body=nt to avoid forward-biased junction)
M6 nt phib gate nt pch L=28n W=1u

.ends bootstrap_sw


***** Testbench *****
.param VDD=0.9

* Power supply
VVDD vdd 0 DC VDD

* Input: 5MHz sine, full swing 0 ~ 0.9V
VIN in 0 SIN(0.45 0.44 5e6 0 0)

* Sampling clock: 100MHz (5ns sample + 5ns hold)
* Rise/fall = 100ps
VPHI phi 0 PULSE(0 VDD 0 100p 100p 4.8n 10n)

* Bootstrap switch instance
X1 in out phi vdd 0 bootstrap_sw

* Sampling capacitor (DAC total capacitance ~200fF)
Csamp out 0 200f

*** Transient simulation: 1us (5 input cycles, 100 sampling cycles) ***
.tran 10p 1u

.control
run
wrdata bootstrap_out v(in) v(out) v(phi) v(x1.gate) v(x1.nt) v(x1.nb) v(x1.m5g)
echo "Bootstrap switch simulation done."
.endc

.end
