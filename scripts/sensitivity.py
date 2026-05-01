#!/usr/bin/env python3
"""Score-weight sensitivity sweep.

Reads the eval JSON for each test circuit, derives sub-scores using the
same formulas as src/eval/score.rs, then sweeps each weight ±delta while
renormalizing the others. Reports per-circuit overall scores and the
rank changes under each perturbation.
"""
import json
import os
import sys
from copy import deepcopy

WEIGHTS = {
    "overlap":          0.20,
    "crossings":        0.15,
    "aspect_ratio":     0.20,
    "wire_length":      0.10,
    "label_ratio":      0.10,
    "symmetry":         0.15,
    "power_convention": 0.10,
}

CIRCUITS = [
    "01_voltage_divider",
    "02_rc_lowpass_filter",
    "03_halfwave_rectifier",
    "04_nmos_common_source",
    "05_nmos_current_mirror",
    "06_bjt_diff_pair",
    "07_two_stage_opamp",
    "08_bandgap_reference",
    "09_inverter_chain_hier",
    "10_opamp_feedback_hier",
    "11_rlc_controlled_sources",
]

def subscores(r):
    """Compute the seven sub-scores from a raw eval report (same formulas
    as src/eval/score.rs)."""
    overlap = 1.0 if r["component_overlap"]["overlap_count"] == 0 else 0.0
    crossings = 1.0 / (1.0 + r["wire_crossings"]["crossing_count"])
    ar = r["bounding_box"]["aspect_ratio"]
    if ar <= 2.5:
        aspect_ratio = 1.0
    elif ar <= 5.0:
        aspect_ratio = 1.0 - (ar - 2.5) / 2.5 * 0.5
    elif ar <= 10.0:
        aspect_ratio = 0.5 - (ar - 5.0) / 5.0 * 0.3
    else:
        aspect_ratio = max(0.0, 0.2 - (ar - 10.0) / 40.0 * 0.2)
    cc = max(1, r["bounding_box"]["component_count"])
    ratio = r["wire_length"]["total_length"] / (cc * 100.0)
    wire_length = 1.0 if ratio <= 1.0 else 1.0 / ratio
    lr = r["label_usage"]["label_to_wire_ratio"]
    if lr <= 0.0 or lr == float("inf"):
        label_ratio = 1.0 if r["label_usage"]["direct_wires"] > 0 else 0.5
    else:
        label_ratio = 1.0 / (1.0 + lr * 2.0)
    symmetry = r["symmetry"]["overall_score"]
    power_convention = r["power_convention"]["score"]
    return {
        "overlap": overlap, "crossings": crossings, "aspect_ratio": aspect_ratio,
        "wire_length": wire_length, "label_ratio": label_ratio,
        "symmetry": symmetry, "power_convention": power_convention,
    }

def overall(subs, weights):
    return sum(weights[k] * subs[k] for k in weights)

def perturb_weights(base, key, delta):
    """Add `delta` to `base[key]` and proportionally renormalize the
    others so the total stays at 1.0. Returns None if the resulting
    weight would go below 0 or above 1."""
    new = deepcopy(base)
    new[key] = base[key] + delta
    if new[key] < 0 or new[key] > 1:
        return None
    others_total = sum(v for k, v in base.items() if k != key)
    if others_total <= 0:
        return None
    target_others = 1.0 - new[key]
    scale = target_others / others_total
    for k in new:
        if k != key:
            new[k] = base[k] * scale
    return new

def rank_with(weights, subs_per_circuit):
    """Return (sorted list of (circuit, score), rank_dict) under given weights."""
    scores = [(c, overall(subs_per_circuit[c], weights)) for c in CIRCUITS]
    sorted_scores = sorted(scores, key=lambda x: -x[1])
    ranks = {c: i for i, (c, _) in enumerate(sorted_scores)}
    return sorted_scores, ranks

def main(eval_dir):
    subs_per_circuit = {}
    for c in CIRCUITS:
        path = os.path.join(eval_dir, f"{c}_eval.json")
        with open(path) as f:
            r = json.load(f)
        subs_per_circuit[c] = subscores(r)

    # Baseline
    print("=== Baseline weights ===")
    for k, v in WEIGHTS.items():
        print(f"  {k:>18}: {v:.2f}")

    base_sorted, base_ranks = rank_with(WEIGHTS, subs_per_circuit)
    print("\nBaseline ranking (highest score first):")
    for i, (c, s) in enumerate(base_sorted, 1):
        print(f"  {i:>2}. {c:<28} {s:.3f}")

    # Sweep each weight ± deltas
    deltas = [-0.10, -0.05, +0.05, +0.10]
    print("\n=== One-at-a-time perturbation ±0.05 / ±0.10 ===")
    print()
    print(f"{'weight':<18}{'delta':>7}  ranking changes (rank_at_delta - rank_baseline) per circuit")

    for k in WEIGHTS:
        for d in deltas:
            new_w = perturb_weights(WEIGHTS, k, d)
            if new_w is None:
                continue
            _, ranks = rank_with(new_w, subs_per_circuit)
            diffs = []
            for c in CIRCUITS:
                rd = ranks[c] - base_ranks[c]
                if rd != 0:
                    diffs.append(f"{c[:6]}{'+' if rd > 0 else ''}{rd}")
            diff_str = ", ".join(diffs) if diffs else "(no change)"
            print(f"{k:<18}{d:+7.2f}  {diff_str}")

    # Compute rank stability metric: max swap distance per circuit across all sweeps
    print("\n=== Per-circuit rank volatility ===")
    print("(max absolute rank shift observed across all perturbations)")
    max_shift = {c: 0 for c in CIRCUITS}
    for k in WEIGHTS:
        for d in deltas:
            new_w = perturb_weights(WEIGHTS, k, d)
            if new_w is None:
                continue
            _, ranks = rank_with(new_w, subs_per_circuit)
            for c in CIRCUITS:
                shift = abs(ranks[c] - base_ranks[c])
                if shift > max_shift[c]:
                    max_shift[c] = shift
    for c in CIRCUITS:
        bar = "#" * max_shift[c]
        print(f"  {c:<28} max_shift={max_shift[c]:>2} {bar}")

    # Top-N stability: how often does the same circuit stay in top 3?
    print("\n=== Top-3 stability ===")
    top3_baseline = set(c for c, _ in base_sorted[:3])
    perturbations_count = 0
    top3_changes = 0
    for k in WEIGHTS:
        for d in deltas:
            new_w = perturb_weights(WEIGHTS, k, d)
            if new_w is None:
                continue
            perturbations_count += 1
            sorted_, _ = rank_with(new_w, subs_per_circuit)
            new_top3 = set(c for c, _ in sorted_[:3])
            if new_top3 != top3_baseline:
                top3_changes += 1
    print(f"  baseline top-3: {sorted(top3_baseline)}")
    print(f"  changes under {perturbations_count} perturbations: {top3_changes}")

    # Score range — total score variance per circuit across all perturbations
    print("\n=== Score range per circuit (across perturbations) ===")
    print(f"  {'circuit':<28}  {'min':>6} {'baseline':>9} {'max':>6}  range")
    per_circuit_scores = {c: [] for c in CIRCUITS}
    for k in WEIGHTS:
        for d in deltas:
            new_w = perturb_weights(WEIGHTS, k, d)
            if new_w is None:
                continue
            for c in CIRCUITS:
                per_circuit_scores[c].append(overall(subs_per_circuit[c], new_w))
    for c in CIRCUITS:
        ss = per_circuit_scores[c]
        base = overall(subs_per_circuit[c], WEIGHTS)
        print(f"  {c:<28}  {min(ss):>6.3f} {base:>9.3f} {max(ss):>6.3f}  Δ={max(ss)-min(ss):.3f}")

if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "/tmp/sweep")
