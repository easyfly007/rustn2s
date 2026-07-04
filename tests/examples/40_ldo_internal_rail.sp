* LDO with internal regulated rail powering a load inverter [n2s test case 40]
*
* === n2s test-case metadata ===
* Source:       hand-written probe for audit item C1 (docs/overfitting_audit.md)
* Circuit type: Pass-device LDO (error amp + PMOS pass FET) whose OUTPUT
*               net `vreg` is the supply rail for a downstream inverter
* Devices:      15 -- 9 MOSFET, 2 R, 1 C, 3 V/I sources
* MOSFETs:      9 (5 NMOS, 4 PMOS), textbook nch/pch model names
* Notes:        C1 target: `vreg` is a POWER net for MLP/MLN but is not in
*               the hardcoded rail list and is not a V/I-source terminal --
*               the tool cannot discover it. Observed on 2026-07-04 without
*               the directive below: the MLP/MLN inverter went unrecognized
*               (matcher needs PMOS source on a power net), got laid out
*               HORIZONTALLY, and Tier 1 power_convention failed (MPASS
*               below MLN). The `n2s: power_net` comment directive is the
*               supported fix; delete the line below to reproduce.
* Added:        2026-07-04  (batch 3: audit-gap probes)
* ===============================

* n2s: power_net vreg

* Supplies and reference
VDD vdd 0 DC 3.3
VREF vref 0 DC 1.2
IB ibias 0 DC 10u

* Error amplifier: NMOS diff pair + PMOS mirror load, tail from ibias mirror
MN1 n1 vref tail vss nch W=10u L=1u
MN2 n2 fb   tail vss nch W=10u L=1u
MP1 n1 n1 vdd vdd pch W=20u L=1u
MP2 n2 n1 vdd vdd pch W=20u L=1u
MNT tail ibias vss vss nch W=5u L=2u
MNB ibias ibias vss vss nch W=5u L=2u

* PMOS pass device: gate from error amp, drain is the regulated rail
MPASS vreg n2 vdd vdd pch W=200u L=0.5u

* Feedback divider
R1 vreg fb 100k
R2 fb vss 100k

* Output cap on the regulated rail
CL vreg vss 1u

* Load inverter powered from the REGULATED rail (vreg = its VDD)
MLP out in vreg vreg pch W=4u L=0.5u
MLN out in vss  vss  nch W=2u L=0.5u
