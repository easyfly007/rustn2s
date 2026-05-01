#!/usr/bin/env python3
"""Per-circuit sub-score profile and limiting-factor analysis.

For each circuit:
  1. Compute the seven sub-scores.
  2. Identify the "dominating limiter" — the lowest sub-score, weighted.
  3. Compute "potential ceiling" — what would overall be if every
     sub-score below 0.95 were lifted to 1.0?
  4. Categorize:
       perfect   — overall ≥ 0.99 (no work to do)
       ar-only   — only aspect_ratio < 0.95 (metric bias suspect)
       real      — other sub-scores < 0.95 (genuine layout issue)
       both      — aspect_ratio AND another sub-score below 0.95

Used to decide: which circuits' low scores reflect real problems vs.
overfitting of the score formula? Output is a triage table you can
quote in a metric-reform doc.
"""
import json
import os
import sys

WEIGHTS = {
    "overlap":          0.20,
    "crossings":        0.15,
    "aspect_ratio":     0.20,
    "wire_length":      0.10,
    "label_ratio":      0.10,
    "symmetry":         0.15,
    "power_convention": 0.10,
}


def subscores(r):
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


def categorize(subs, threshold=0.95):
    """Classify a circuit by which sub-scores fall below threshold."""
    weak = [k for k, v in subs.items() if v < threshold]
    if not weak:
        return "perfect", []
    # Distinguish ar-only from "real" weaknesses
    non_ar_weak = [k for k in weak if k != "aspect_ratio"]
    if not non_ar_weak:
        return "ar-only", weak
    if "aspect_ratio" in weak:
        return "both", weak
    return "real", weak


def potential_ceiling(subs, weights, threshold=0.95):
    """If every sub-score below threshold were 1.0, what would overall be?"""
    lifted = {k: (1.0 if v < threshold else v) for k, v in subs.items()}
    return overall(lifted, weights)


def potential_no_ar(subs, weights):
    """Drop aspect_ratio entirely (set to 1.0). Tells us how much
    aspect_ratio alone is dragging the circuit down."""
    no_ar = dict(subs)
    no_ar["aspect_ratio"] = 1.0
    return overall(no_ar, weights)


def main(eval_dir):
    circuits = sorted(
        f[:-len("_eval.json")] for f in os.listdir(eval_dir)
        if f.endswith("_eval.json")
    )

    rows = []
    for c in circuits:
        with open(os.path.join(eval_dir, f"{c}_eval.json")) as f:
            r = json.load(f)
        subs = subscores(r)
        rows.append((c, subs, r))

    # Header
    print(f"{'circuit':<32} {'overall':>7} {'no-ar':>6} {'ceil':>6} {'category':<10} {'limiters':<60}")
    print("-" * 120)

    cat_counts = {"perfect": 0, "ar-only": 0, "real": 0, "both": 0}
    for c, subs, r in rows:
        ov = overall(subs, WEIGHTS)
        no_ar = potential_no_ar(subs, WEIGHTS)
        ceil = potential_ceiling(subs, WEIGHTS)
        cat, weak = categorize(subs)
        cat_counts[cat] += 1
        weak_str = ", ".join(f"{k[:4]}={subs[k]:.2f}" for k in weak)
        print(f"{c:<32} {ov:>7.3f} {no_ar:>6.3f} {ceil:>6.3f} {cat:<10} {weak_str:<60}")

    print()
    print("=== Category counts ===")
    for cat, count in cat_counts.items():
        print(f"  {cat:<10} {count}")

    # Specific triage groups
    print()
    print("=== ar-only circuits (probably metric bias, not real bugs) ===")
    for c, subs, r in rows:
        cat, _ = categorize(subs)
        if cat == "ar-only":
            ar_score = subs["aspect_ratio"]
            ar_value = r["bounding_box"]["aspect_ratio"]
            no_ar = potential_no_ar(subs, WEIGHTS)
            print(f"  {c:<32} ar={ar_value:.1f} ar_score={ar_score:.2f}  if-ar-removed→{no_ar:.3f}")

    print()
    print("=== real-issue circuits (genuine layout problems) ===")
    for c, subs, r in rows:
        cat, weak = categorize(subs)
        if cat == "real":
            non_ar = [k for k in weak if k != "aspect_ratio"]
            details = ", ".join(f"{k}={subs[k]:.2f}" for k in non_ar)
            print(f"  {c:<32}  weaknesses: {details}")

    print()
    print("=== both (ar-limited AND real issues) ===")
    for c, subs, r in rows:
        cat, weak = categorize(subs)
        if cat == "both":
            non_ar = [k for k in weak if k != "aspect_ratio"]
            details = ", ".join(f"{k}={subs[k]:.2f}" for k in non_ar)
            print(f"  {c:<32}  ar={r['bounding_box']['aspect_ratio']:.1f}  + {details}")

    # Per-sub-score weakness frequency: how often does each sub-score
    # appear below threshold across the suite?
    print()
    print("=== Sub-score weakness frequency (below 0.95) ===")
    freq = {k: 0 for k in WEIGHTS}
    for c, subs, _ in rows:
        for k, v in subs.items():
            if v < 0.95:
                freq[k] += 1
    for k, count in sorted(freq.items(), key=lambda x: -x[1]):
        bar = "#" * count
        n = len(rows)
        pct = 100 * count / n if n else 0
        print(f"  {k:<18} {count:>3}/{n}  ({pct:>4.0f}%)  {bar}")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "/tmp/sweep")
