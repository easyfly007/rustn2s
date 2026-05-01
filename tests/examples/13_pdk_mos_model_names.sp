* Industrial PDK MOS Model Names
* Stress-test for the MOS keyword inference (audit item C2):
* infer_mos_type only matches "nch" / "nmos" / "pch" / "pmos" substrings
* in the model name. Real PDK names like sky130_fd_pr__nfet_01v8 or
* g45n1svt_pmos2v_18 may or may not match. Falls back to bulk-node-based
* inference, which works when bulk is a power net but breaks if bulk is
* a biased (non-power) node.
V1 vdd 0 1.8
V2 vin 0 AC=0.5
* M1 has bulk == 0, so bulk-fallback recovers NMOS even with non-keyword
* model name.
M1 vd vin 0 0 sky130_fd_pr__nfet_01v8 W=2u L=0.18u
* M2 is a PMOS with bulk == vdd, recovered by fallback.
M2 vout vd vdd vdd sky130_fd_pr__pfet_01v8 W=4u L=0.18u
* M3 has the same model family as M1 but a non-power bulk (vbias). The
* keyword check fails, the bulk-fallback fails (vbias is unknown), so
* the parser defaults to NMOS — happens to be correct here but
* demonstrates a brittle path.
M3 vout2 vin vbias vbias g45n1svt W=1u L=0.18u
R1 vdd vd 10k
R2 vd vout 1k
R3 vdd vout2 5k
* Constant bias for M3's bulk (in real circuits this would be a
* non-trivial network).
V3 vbias 0 0.5
