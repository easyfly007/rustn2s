# Test Examples

This document describes the test netlists in `tests/examples/`. Each example includes a circuit description, netlist walkthrough, and the command to generate a schematic.

All commands assume you have built the binary:

```bash
cargo build --release
```

---

## Simple Circuits

### 01 — Voltage Divider

**File:** `tests/examples/01_voltage_divider.sp`

A basic resistive voltage divider. A 5V source drives two equal 10k resistors in series, producing 2.5V at the midpoint.

```spice
* Voltage Divider
V1 vin 0 5          ; 5V DC source between vin and ground
R1 vin vout 10k     ; Upper resistor: vin → vout
R2 vout 0 10k       ; Lower resistor: vout → ground
```

**Schematic structure:** V1 on the left, R1-R2 divider chain, vout at the midpoint.

**Devices:** V, R (2)

```bash
n2s tests/examples/01_voltage_divider.sp -o 01_voltage_divider.svg
```

---

### 02 — RC Low-Pass Filter

**File:** `tests/examples/02_rc_lowpass_filter.sp`

A first-order passive low-pass filter. The cutoff frequency is `1/(2*pi*R*C) = 1/(2*pi*1k*10n) ≈ 15.9 kHz`.

```spice
* RC Low-Pass Filter
V1 in 0 AC=1        ; AC stimulus, 1V amplitude
R1 in out 1k        ; Series resistor
C1 out 0 10n        ; Shunt capacitor to ground
```

**Schematic structure:** Signal flows left-to-right through R1, C1 shunts to ground at the output node.

**Devices:** V, R, C

```bash
n2s tests/examples/02_rc_lowpass_filter.sp -o 02_rc_lowpass_filter.svg
```

---

### 03 — Half-Wave Rectifier

**File:** `tests/examples/03_halfwave_rectifier.sp`

A diode half-wave rectifier with an RC smoothing filter. D1 passes positive half-cycles, C1 smooths the output, and R1 serves as the load.

```spice
* Half-Wave Rectifier
V1 in 0 SIN(0,5,1k) ; 1 kHz, 5V peak sine wave
D1 in out DMOD       ; Rectifier diode (anode=in, cathode=out)
R1 out 0 1k          ; Load resistor
C1 out 0 10u         ; Smoothing capacitor
```

**Schematic structure:** V1 → D1 → parallel R1/C1 to ground.

**Devices:** V, D, R, C

```bash
n2s tests/examples/03_halfwave_rectifier.sp -o 03_halfwave_rectifier.svg
```

---

## Medium Circuits

### 04 — NMOS Common-Source Amplifier

**File:** `tests/examples/04_nmos_common_source.sp`

A single-stage NMOS amplifier with resistive biasing and AC coupling.

```spice
* NMOS Common-Source Amplifier
V1 vdd 0 3.3                        ; 3.3V supply
V2 vin 0 AC=1                       ; AC input signal
R1 vdd vdrain 5k                    ; Drain load resistor
R2 vdd vgate 100k                   ; Upper bias resistor (voltage divider)
R3 vgate 0 50k                      ; Lower bias resistor (sets gate DC bias)
R4 vsource 0 1k                     ; Source degeneration resistor
C1 vin vgate 1u                     ; Input coupling cap (blocks DC)
C2 vdrain vout 1u                   ; Output coupling cap (blocks DC)
C3 vsource 0 10u                    ; Source bypass cap (restores AC gain)
M1 vdrain vgate vsource 0 nch W=10u L=1u  ; NMOS: drain gate source bulk
```

**Circuit description:**
- R2/R3 set the gate bias point via a voltage divider
- C1 AC-couples the input signal to the gate
- M1 amplifies: small gate voltage changes produce large drain current changes through R1
- R4 provides DC stabilization; C3 bypasses it at AC for full gain
- C2 AC-couples the amplified output

**Devices:** V (2), R (4), C (3), M (1) — 10 total

```bash
n2s tests/examples/04_nmos_common_source.sp -o 04_nmos_common_source.svg
```

---

### 05 — NMOS Current Mirror

**File:** `tests/examples/05_nmos_current_mirror.sp`

A 1:1:2 NMOS current mirror. A reference current is set by I1 through diode-connected M1. M2 copies the current 1:1, and M3 copies at 2:1 (double width).

```spice
* NMOS Current Mirror
V1 vdd 0 3.3                    ; Supply
I1 vdd diode 100u               ; 100uA reference current
M1 diode diode 0 0 nch W=10u L=2u  ; Diode-connected reference (drain=gate)
M2 out diode 0 0 nch W=10u L=2u    ; Mirror output 1 (1:1, same W/L)
M3 out2 diode 0 0 nch W=20u L=2u   ; Mirror output 2 (2:1, double W)
R1 vdd out 10k                  ; Load for mirror output 1
R2 vdd out2 10k                 ; Load for mirror output 2
```

**Key insight:** M1 has drain=gate ("diode-connected"), establishing a Vgs that M2 and M3 share. Current scales with W/L ratio: M2 mirrors 100uA, M3 mirrors 200uA.

**Devices:** V, I, M (3), R (2) — 7 total

```bash
n2s tests/examples/05_nmos_current_mirror.sp -o 05_nmos_current_mirror.svg
```

---

### 06 — BJT Differential Pair

**File:** `tests/examples/06_bjt_diff_pair.sp`

A classic NPN differential amplifier with resistive loads and a tail current source.

```spice
* BJT Differential Pair
V1 vcc 0 5                  ; 5V supply
V2 inp 0 AC=0.5             ; Positive input (differential)
V3 inm 0 AC=-0.5            ; Negative input (differential)
I1 tail 0 1m                ; 1mA tail current source
Q1 out1 inp tail NPN_MOD    ; Left transistor  (C=out1, B=inp, E=tail)
Q2 out2 inm tail NPN_MOD    ; Right transistor (C=out2, B=inm, E=tail)
R1 vcc out1 5k              ; Left collector load
R2 vcc out2 5k              ; Right collector load
```

**Circuit description:**
- I1 sets the total bias current (1mA), split between Q1 and Q2
- Differential input (V2-V3) steers current between the two branches
- Output is taken differentially between out1 and out2
- Gain ≈ gm * Rc where gm = Ic/(2*Vt)

**Devices:** V (3), I, Q (2), R (2) — 8 total

```bash
n2s tests/examples/06_bjt_diff_pair.sp -o 06_bjt_diff_pair.svg
```

---

## Complex Circuits

### 07 — Two-Stage CMOS Op-Amp

**File:** `tests/examples/07_two_stage_opamp.sp`

A Miller-compensated two-stage CMOS operational amplifier — the most common analog building block.

```spice
* Two-Stage CMOS Op-Amp
V1 vdd 0 3.3
V2 inp 0 AC=0.5
V3 inm 0 AC=-0.5
* Input diff pair (PMOS)
M1 drain1 inp tail vdd pch W=20u L=1u   ; PMOS input pair (left)
M2 drain2 inm tail vdd pch W=20u L=1u   ; PMOS input pair (right)
* Tail current source
M3 tail bias_p vdd vdd pch W=40u L=2u   ; PMOS tail current source
* NMOS active load (current mirror)
M4 drain1 drain1 0 0 nch W=10u L=1u     ; Diode-connected (reference)
M5 drain2 drain1 0 0 nch W=10u L=1u     ; Mirror output
* Second stage: common-source
M6 vout drain2 0 0 nch W=40u L=1u       ; NMOS gain stage
M7 vout bias_p vdd vdd pch W=80u L=2u   ; PMOS active load
* Bias generation
M8 bias_p bias_p vdd vdd pch W=20u L=2u ; Diode-connected bias
I1 bias_p 0 50u                          ; Bias reference current
* Miller compensation
C1 drain2 vout 2p                        ; Compensation cap
R1 drain2 comp_mid 500                   ; Nulling resistor
C2 comp_mid vout 1p                      ; Additional compensation
* Load
C3 vout 0 5p                             ; Output load capacitance
```

**Architecture:**
1. **Stage 1** — PMOS diff pair (M1/M2) with NMOS active load mirror (M4/M5). Provides differential-to-single-ended conversion and first-stage gain.
2. **Stage 2** — Common-source amplifier (M6/M7). Provides second-stage gain and output swing.
3. **Bias** — M8 (diode-connected) + I1 generate bias_p, shared by M3 and M7.
4. **Compensation** — Miller cap C1 with nulling resistor R1 ensures stability by splitting the two gain poles.

**Devices:** V (3), M (8), I, C (3), R — 16 total

```bash
n2s tests/examples/07_two_stage_opamp.sp -o 07_two_stage_opamp.svg
```

---

### 08 — Bandgap Voltage Reference

**File:** `tests/examples/08_bandgap_reference.sp`

A bandgap reference circuit generating a temperature-stable voltage (~1.2V) using the complementary temperature coefficients of Vbe and delta-Vbe.

```spice
* Bandgap Voltage Reference
V1 vdd 0 3.3
* PMOS current mirror
M1 branch1 bias vdd vdd pch W=20u L=2u  ; Mirror leg 1
M2 branch2 bias vdd vdd pch W=20u L=2u  ; Mirror leg 2
M3 bias bias vdd vdd pch W=20u L=2u     ; Diode-connected reference
* BJT pair
Q1 0 branch1 e1 NPN_MOD                 ; BJT 1 (collector to ground)
Q2 0 branch2 e2 NPN_MOD                 ; BJT 2 (collector to ground)
* Resistors
R1 e1 0 10k          ; Emitter resistor for Q1
R2 e2 0 1k           ; Emitter resistor for Q2 (different from R1 → delta-Vbe)
R3 branch1 e1 5k     ; Sensing resistor
* Feedback amplifier
E1 bias 0 branch1 branch2 1000  ; VCVS models op-amp feedback (high gain)
* Output
R4 vdd vref 20k                 ; Output resistor
M4 vref bias vdd vdd pch W=20u L=2u  ; Output mirror transistor
C1 vref 0 10p                   ; Bypass capacitor
```

**Key principle:** Q1 and Q2 run at different current densities (set by R1 vs R2). The difference in Vbe (PTAT — proportional to absolute temperature) is combined with Vbe itself (CTAT — complementary to temperature) to produce a temperature-independent reference.

**Devices:** V, M (4), Q (2), R (4), E, C — 13 total

```bash
n2s tests/examples/08_bandgap_reference.sp -o 08_bandgap_reference.svg
```

---

## Hierarchical Circuits

### 09 — Inverter Chain (Nested Subcircuits)

**File:** `tests/examples/09_inverter_chain_hier.sp`

Demonstrates hierarchical design with nested subcircuits: `INV` (inverter) is used inside `BUF` (buffer), and both are instantiated at the top level.

```spice
* Inverter Chain with Subcircuit

* --- Subcircuit: CMOS Inverter ---
.subckt INV in out vdd vss
M1 out in vdd vdd pch W=20u L=1u    ; PMOS pull-up
M2 out in vss vss nch W=10u L=1u    ; NMOS pull-down
.ends INV

* --- Subcircuit: Buffer (two inverters) ---
.subckt BUF in out vdd vss
X1 in mid vdd vss INV               ; First inverter
X2 mid out vdd vss INV              ; Second inverter (restores polarity)
.ends BUF

* --- Top Level ---
V1 vdd 0 3.3                        ; Supply
X1 input net1 vdd 0 INV             ; Inverter stage
X2 net1 net2 vdd 0 BUF              ; Buffer stage (2 inverters inside)
X3 net2 output vdd 0 INV            ; Final inverter stage
C1 output 0 1p                      ; Load capacitance
```

**Hierarchy:**
```
Top Level
├── X1 (INV) ─── M1, M2
├── X2 (BUF)
│   ├── X1 (INV) ─── M1, M2
│   └── X2 (INV) ─── M1, M2
├── X3 (INV) ─── M1, M2
└── C1
```

Total path: `input → INV → BUF(INV→INV) → INV → output` — 4 inversions, so output = input.

**Devices:** V, X (3 top-level, 2 inside BUF), M (8 total when flattened), C

```bash
n2s tests/examples/09_inverter_chain_hier.sp -o 09_inverter_chain_hier.svg
```

---

### 10 — Op-Amp in Non-Inverting Feedback (Subcircuit)

**File:** `tests/examples/10_opamp_feedback_hier.sp`

A complete op-amp defined as a `.subckt`, then instantiated in a non-inverting amplifier configuration with resistive feedback.

```spice
* Op-Amp Feedback Circuit Using Subcircuit

* --- Subcircuit: Two-Stage Op-Amp ---
.subckt OPAMP inp inm vdd vss out
M1 d1 inp tail vdd pch W=20u L=1u   ; Diff pair (left)
M2 d2 inm tail vdd pch W=20u L=1u   ; Diff pair (right)
M3 tail biasp vdd vdd pch W=40u L=2u ; Tail current source
M4 d1 d1 vss vss nch W=10u L=1u     ; Active load (diode)
M5 d2 d1 vss vss nch W=10u L=1u     ; Active load (mirror)
M6 out d2 vss vss nch W=40u L=1u    ; Output stage (NMOS)
M7 out biasp vdd vdd pch W=80u L=2u ; Output stage (PMOS)
C1 d2 out 2p                         ; Miller compensation
I1 biasp vss 50u                     ; Bias current
M8 biasp biasp vdd vdd pch W=20u L=2u ; Bias diode
.ends OPAMP

* --- Top Level: Non-Inverting Amplifier ---
V1 vdd 0 3.3                         ; Supply
V2 vsig 0 AC=1                       ; Input signal
X1 vsig fb vdd 0 vout OPAMP          ; Op-amp instance
R1 vout fb 10k                        ; Feedback resistor (top)
R2 fb 0 10k                           ; Feedback resistor (bottom)
C1 vout 0 5p                          ; Load capacitance
```

**Feedback analysis:**
- Non-inverting config: `inp=vsig`, `inm=fb`
- Gain = 1 + R1/R2 = 1 + 10k/10k = **2x**
- The OPAMP subcircuit contains a full two-stage Miller-compensated design (8 MOSFETs)

**Devices:** V (2), X, R (2), C (top-level); M (8), I, C (inside subcircuit) — 15 total

```bash
n2s tests/examples/10_opamp_feedback_hier.sp -o 10_opamp_feedback_hier.svg
```

---

## Full Device Coverage

### 11 — RLC Filter with All Controlled Source Types

**File:** `tests/examples/11_rlc_controlled_sources.sp`

A test netlist exercising all four controlled source types (E, G, H, F) along with an RLC filter, ensuring every supported device type is covered.

```spice
* RLC and Controlled Source Test
V1 in 0 AC=1
* Series RLC filter
R1 in n1 100         ; Series resistance
L1 n1 n2 10m         ; Series inductance (10mH)
C1 n2 0 1u           ; Shunt capacitance (resonant freq ≈ 1.6 kHz)

* VCVS — Voltage-Controlled Voltage Source
E1 buf_out 0 n2 0 2  ; Output = 2 * V(n2): voltage buffer with gain

* VCCS — Voltage-Controlled Current Source
G1 vdd gm_out n2 0 1m   ; Output current = 1mA/V * V(n2): transconductor
R2 vdd gm_out 1k        ; Load for G1

* CCVS — Current-Controlled Voltage Source
V2 sense_in sense_out 0 ; Zero-volt source to sense current
H1 hout 0 V2 500        ; Output = 500 * I(V2): transresistance amp
R3 sense_in 0 100       ; Load to set sensed current

* CCCS — Current-Controlled Current Source
V3 fsense_in fsense_out 0  ; Zero-volt current sensor
F1 vdd fout V3 10           ; Output = 10 * I(V3): current amplifier
R4 fsense_in 0 100          ; Load to set sensed current
R5 vdd fout 1k              ; Load for F1

V4 vdd 0 5                  ; Supply for active loads
```

**Controlled source summary:**

| Type | Instance | Function | Equation |
|------|----------|----------|----------|
| E (VCVS) | E1 | Voltage buffer with gain | Vout = 2 * V(n2) |
| G (VCCS) | G1 | Transconductor | Iout = 1mA/V * V(n2) |
| H (CCVS) | H1 | Transresistance amplifier | Vout = 500 * I(V2) |
| F (CCCS) | F1 | Current amplifier | Iout = 10 * I(V3) |

**Devices:** V (4), R (5), L, C, E, G, H, F — 14 total (all 13 device types except M, Q, D, X)

```bash
n2s tests/examples/11_rlc_controlled_sources.sp -o 11_rlc_controlled_sources.svg
```

---

## Expanded Adversarial Suite (12–25)

Circuits 01–11 come from the original C++ MySchematic suite. Circuits 12–25 were added on 2026-05-01 to break the blind spots the [overfitting audit](overfitting_audit.md) predicted — the original 11 saturated three of the score's sub-dimensions and never exercised several real-world conventions. The expansion paid off: these 14 circuits exposed **four real bugs** and **two evaluator gaps** (full write-up in [test_set_expansion_findings.md](test_set_expansion_findings.md)). They are intentionally terse stress cases rather than tutorial circuits, so each is summarized in one line.

| # | Circuit | What it stresses | What it revealed |
|---|---------|------------------|------------------|
| 12 | Industrial power names | Power nets named `VBAT`/`VIO` instead of `VDD`/`GND` | Clean — the V-source-terminal rule rescues non-standard power names (1.000) |
| 13 | PDK MOS model names | NMOS/PMOS split across DAG layers, foundry-style model names | **Bug 1** — PMOS lands below NMOS across layers; `power_convention=0.0` (0.900) |
| 14 | Disconnected filters | Two fully independent sub-circuits, no shared nets | **Bug 2** — two isolated sources collapse to identical (x, y); `overlap=0.0` (0.800) |
| 15 | Pi attenuator | Short nets routed entirely by direct wires | **Gap 2** — `found_net_count` undercounts wire-only nets (cosmetic) |
| 16 | Inverter chain (5-stage) | Long horizontal cascade | **Bug 4** — wide-but-correct layout penalized by `aspect_ratio`; metric bias, not a placement bug (0.723) |
| 17 | Folded-cascode op-amp | Genuinely complex analog topology | Real routing complexity, not a bug (0.750) |
| 18 | Star fanout | One net driving many sinks | Clean (0.994) — MST routing handles high fanout |
| 19 | Degenerated diff pair | Diff pair with source-degeneration resistors | Clean (0.903) |
| 20 | Asymmetric pair | Two devices that look like a pair but differ in attributes | **Gap 1** — `symmetry` is vacuously 1.000 when no true pairs exist |
| 21 | Deep signal chain | 10 RC stages in series | **Bug 4** (same shape bias as 16) — long chain, 0.866 |
| 22 | Three isolated sources | Three V/R source pairs | Clean (0.950) — pairs cluster into HAC groups, so they never become "isolated" (does NOT repro Bug 2) |
| 23 | PMOS-input inverter | Inverter with PMOS as the input device | Clean (0.959) |
| 24 | Dense amp array | Many amplifier blocks packed together | Clean (0.938) — scale/density stress |
| 25 | Subckt array | `.subckt` defs + top-level X instances and loads | **Bug 3** — default mode rendered only the first subckt's interior, dropping 6 of 8 top-level items (now fixed in `lib.rs`) |

Baseline scores above use the legacy weighted `compute_score`; under the [two-tier metric reform](architecture.md#two-tier-evaluation-api-2026-05-01-metric-reform) the shape-bias circuits (02, 03, 16, 21) report a low `shape` signal but a high `quality`, and the safety bugs (13, 14) surface as Tier 1 failures rather than fractional scores.

---

## Real-World Circuits (26–30, from the `myadc` SAR-ADC design)

Circuits 01–25 are hand-written. Circuits 26–30 were lifted on 2026-06-15 from the `myadc` repository — a real 8-bit SAR ADC implemented in both PTM 28nm and SKY130 — to validate `n2s` against netlists nobody wrote *for* `n2s`. Each `.sp` carries a metadata header (source path, circuit type, device count, MOSFET count). Importing them immediately surfaced a real parser bug: ngspice `.control ... .endc` blocks were not skipped, so commands like `run` and `echo` were mis-parsed as R and E devices (fixed; see the `test_control_block_is_skipped` regression test).

| # | Circuit | Source (myadc) | Type | Devices | MOSFETs | What it adds |
|---|---------|----------------|------|:-------:|:-------:|--------------|
| 26 | Dynamic comparator (28nm) | `ptm28nm/comparator/double_tail_comp.spice` | Double-tail clocked comparator (pre-amp + cross-coupled latch) | 23 (15 M, 4 C, 4 V) | 15 (8 N, 7 P) | First real clocked/regenerative analog block; flat netlist |
| 27 | Bootstrap switch (28nm) | `ptm28nm/bootstrap/bootstrap_sw.spice` | Bootstrapped S/H sampling switch — hierarchical | 5 top-level (1 subckt box, 1 C, 3 V); subckt holds 8 M + 2 C + 1 R | 8 (in subckt) | X-instance + local `.subckt` hierarchical rendering |
| 28 | Capacitor DAC (28nm) | `ptm28nm/dac/cap_dac_4p4.spice` | Binary-weighted (4+4 split) capacitor DAC array | 12 (10 C, 2 V) | 0 | Passive cap array; triggered the `.control`-skip bug (`destroy`/`let`) |
| 29 | Async SAR logic (28nm) | `ptm28nm/sar_logic/async_sar.spice` | Asynchronous 8-bit SAR controller (digital, hierarchical) | 92 (88 gate/latch X instances, 2 C, 2 V) | 0 top-level | **Scale stress** — largest case at 92 components; X instances whose `.subckt` defs live in an `.include`'d file; quality floors at 0.25 (router hairball at scale) |
| 30 | Dynamic comparator (sky130) | `sky130/comparator/double_tail_comp.spice` | Same comparator as 26, ported to SKY130 130nm | 24 (16 transistors, 4 C, 4 V) | 16 as X instances (9 nfet, 7 pfet) | PDK device modeling — transistors are `sky130_fd_pr__*fet` subckt instances with no in-file def |

All five pass Tier 1 safety. 26/27/28/30 score `quality ≥ 0.80`; 29 deliberately documents the router's behavior on a 92-component digital block (`cross=0.02`), the concrete scale ceiling STATUS.md asked to probe.

---

## Real-World Circuits, Batch 2 (31–36, from `myadc`)

Added 2026-07-03. Same sourcing rules as 26–30 (real netlists, metadata headers, no `*_pex_*` files). Selection targeted the blind spots STATUS.md left open: bulk-node type inference (C2-adjacent), cyclic connectivity, repetitive arrays, PMOS-input polarity, and true scale.

| # | Circuit | Source (myadc) | Type | Devices | MOSFETs | What it adds |
|---|---------|----------------|------|:-------:|:-------:|--------------|
| 31 | TG master-slave DFF (28nm) | `ptm28nm/layout/tech/schematics/dff.spice` | Transmission-gate DFF, LVS reference, subckt-only | 16 M | 16 (8 N, 8 P) | Bare `nfet`/`pfet` model names force bulk-node type inference; master/slave feedback loops make the graph cyclic. **Found a metric false positive**: stacked CMOS stages in one column tripped `power_convention`'s global pairwise check (fixed — nearest-NMOS pairing) |
| 32 | Comparator clock gen (28nm) | `ptm28nm/layout/tech/schematics/comp_clk_gen.spice` | 8-input NOR/NAND tree + INV, flattened | 30 M | 30 (16 N, 14 P) | Series-stack gate shapes stress HAC clustering and the inverter matcher |
| 33 | Sampling switch array (sky130) | `sky130/layout/sampling_sw/sampling_sw_schematic.spice` | 9 CMOS transmission gates (1 reset + 8 sampling) | 18 X | 18 (9 nfet, 9 pfet) | Highly repetitive TG array; pair-aware Unknown template + label dedup on shared `vin`/`phi` nets |
| 34 | PMOS-input comparator (sky130) | `sky130/comparator/pmos_double_tail_comp.spice` | PMOS-input double-tail comparator (mirror of 26/30) | 24 (16 X, 4 C, 4 V) | 16 (7 nfet, 9 pfet) | Signal flows PMOS→NMOS, opposite of the textbook layout; `.control` block with `foreach` |
| 35 | Bootstrap switch LVS (sky130) | `sky130/layout/bootstrap/bootstrap_schematic.spice` | Abo-Gray bootstrapped switch, flat | 13 X (12 fet + 1 MIM cap) | 12 (7 nfet, 5 pfet) | Body pins wired to *signal* nets (triple-well body=out, iso-nwell body=nt); X-instance MIM cap. NOTE: X instances, so M-card C2 inference is *not* exercised |
| 36 | SAR logic, flat extracted (sky130) | `sky130/layout/sar_logic/sar_logic_flat.spice` | Full 8-bit SAR logic flattened by magic extraction | 684 X | 684 | **Scale stress**, 7× previous max; extraction-style node names with `#` and `.`; runs in ~0.15 s but quality collapses (`cross=0.00`, `wire=0.04`) |

All six pass Tier 1 safety (after the `power_convention` fix motivated by 31). Visual-inspection findings are recorded in `docs/test_set_expansion_findings.md`.

---

## Batch Run

Generate all schematics at once:

```bash
mkdir -p output
for f in tests/examples/*.sp; do
  name=$(basename "$f" .sp)
  n2s "$f" -o "output/${name}.svg"
  echo "Generated: output/${name}.svg"
done
```

Generate both SVG and JSON:

```bash
mkdir -p output
for f in tests/examples/*.sp; do
  name=$(basename "$f" .sp)
  n2s "$f" -o "output/${name}.svg" -o "output/${name}.json"
done
```

---

## Device Type Coverage Matrix

| Device | Type | Examples |
|--------|------|---------|
| M | MOSFET | 04, 05, 07, 08, 09, 10, 13, 16, 17, 20, 23, 24, 25, 26 |
| Q | BJT | 06, 08, 19 |
| R | Resistor | 01–08, 10, 11, 12, 13, 14, 15, 18, 19, 20, 22, 24, 27 |
| C | Capacitor | 02, 03, 04, 07, 08, 09, 10, 11, 12, 14, 16, 17, 21, 23, 24, 25, 26, 27, 28, 29, 30 |
| L | Inductor | 11 |
| D | Diode | 03 |
| V | Voltage source | 01–30 |
| I | Current source | 05, 06, 07, 10, 17, 19, 20 |
| E | VCVS | 08, 11 |
| G | VCCS | 11 |
| H | CCVS | 11 |
| F | CCCS | 11 |
| X | Subcircuit inst | 09, 10, 25, 27, 29, 30 |
| .subckt | Subcircuit def | 09, 10, 25, 27 |
