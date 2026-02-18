#!/usr/bin/env python3
"""
Benchmark regression comparator.

Usage:
    python benchmarks/compare.py <baseline.json> <current.json> [--threshold N]

Exits 0 if no regressions exceed threshold (default 15%).
Exits 1 if any metric regressed by more than threshold.
"""

import json
import sys
import argparse


def key(r):
    return (r["benchmark"], r["file_lines"], r.get("edit_count"))


def load(path):
    with open(path) as f:
        data = json.load(f)
    return {key(r): r["value"] for r in data["results"]}, data


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("baseline")
    parser.add_argument("current")
    parser.add_argument("--threshold", type=float, default=15.0,
                        help="Regression threshold %% (default: 15)")
    args = parser.parse_args()

    baseline_map, baseline_meta = load(args.baseline)
    current_map, current_meta = load(args.current)

    print(f"Baseline : {baseline_meta['version']} @ {baseline_meta['commit']} ({baseline_meta['runner']})")
    print(f"Current  : {current_meta['version']} @ {current_meta['commit']} ({current_meta['runner']})")
    print(f"Threshold: {args.threshold}%\n")

    all_keys = sorted(set(baseline_map) | set(current_map),
                      key=lambda k: (k[0], k[1] or 0, k[2] or 0))

    regressions = []
    rows = []

    for k in all_keys:
        b_val = baseline_map.get(k)
        c_val = current_map.get(k)

        if b_val is None:
            rows.append((k, "-", f"{c_val:.1f}", "N/A", "NEW"))
            continue
        if c_val is None:
            rows.append((k, f"{b_val:.1f}", "-", "N/A", "REMOVED"))
            continue

        pct = (c_val - b_val) / b_val * 100
        flag = ""
        if pct > args.threshold:
            flag = f"REGRESSION (+{pct:.1f}%)"
            regressions.append((k, b_val, c_val, pct))
        elif pct < -5:
            flag = f"improved ({pct:.1f}%)"
        else:
            flag = f"{pct:+.1f}%"

        rows.append((k, f"{b_val:.1f}", f"{c_val:.1f}", f"{pct:+.1f}%", flag))

    # Print table
    bench_w = max(len(str(r[0][0])) for r in rows) + 2
    header = f"{'Benchmark':<{bench_w}} {'Lines':>7} {'Edits':>6} {'Base (µs)':>10} {'Curr (µs)':>10} {'Change':>8}  Note"
    print(header)
    print("-" * len(header))
    for (bname, flines, ecount), base, curr, pct, note in rows:
        ecount_s = str(ecount) if ecount is not None else "-"
        print(f"{bname:<{bench_w}} {flines or 0:>7} {ecount_s:>6} {base:>10} {curr:>10} {pct:>8}  {note}")

    print()
    if regressions:
        print(f"FAIL: {len(regressions)} regression(s) exceed {args.threshold}% threshold:")
        for k, b, c, pct in regressions:
            print(f"  {k[0]} lines={k[1]} edits={k[2]}: {b:.1f} -> {c:.1f} µs (+{pct:.1f}%)")
        sys.exit(1)
    else:
        print(f"OK: no regressions above {args.threshold}% threshold.")
        sys.exit(0)


if __name__ == "__main__":
    main()
