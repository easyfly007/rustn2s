* Foundry-opaque model names with signal-biased bulks [n2s test case 41]
*
* === n2s test-case metadata ===
* Source:       hand-written probe for audit item C2 (docs/overfitting_audit.md)
* Circuit type: Two-stage amplifier with deep-nwell input pair, written in a
*               foundry style whose model names carry NO nmos/pmos keyword
*               (gpdk-like g45n1svt / g45p1svt) and whose bulks sit on BIAS
*               nets, not rails
* Devices:      15 -- 8 MOSFET, 1 R, 1 C, 5 V sources
* MOSFETs:      8. M1/M2 (g45n1svt) bulk=nwell_bias; M5 (g45p1svt)
*               bulk=pwell_bias. Neither the model-name keyword check
*               (nch/nmos/pch/pmos) nor the bulk-rail rescue can type
*               these devices -- BOTH C2 conditions hold.
* KNOWN GAP:    n2s defaults untypable MOSFETs to NMOS, so M5 renders with
*               an NMOS symbol. No metric catches a wrong symbol; this case
*               exists to keep the gap visible. If a real fix lands
*               (e.g. model-card lookup or user hints), update this note.
* Added:        2026-07-04  (batch 3: audit-gap probes)
* ===============================

VDD vdd 0 DC 1.8
VIN inp 0 DC 0.9
VBN nwell_bias 0 DC 0.3
VBP pwell_bias 0 DC 1.5
VIN2 inn 0 DC 0.9

* Input diff pair: deep-nwell NMOS, bulk tied to a bias net (not a rail)
M1 d1 inp cs nwell_bias g45n1svt W=8u L=0.4u
M2 d2 inn cs nwell_bias g45n1svt W=8u L=0.4u

* Mirror load: model name has the pch keyword -> typed correctly
M3 d1 d1 vdd vdd pch W=16u L=0.4u
M4 d2 d1 vdd vdd pch W=16u L=0.4u

* Second-stage PMOS with bulk on a bias net AND an opaque model name:
* the full C2 condition -- n2s cannot know this is a PMOS.
M5 out d2 vdd pwell_bias g45p1svt W=32u L=0.4u

* Tail + bias chain, opaque NMOS names but bulk on ground -> rescued
M6 cs bias 0 0 g45n1svt W=4u L=1u
M7 bias bias 0 0 g45n1svt W=4u L=1u

* Second-stage load and compensation
M8 out bias 0 0 g45n1svt W=8u L=1u
RC d2 cx 2k
CC cx out 1p
