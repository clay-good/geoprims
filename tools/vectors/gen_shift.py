#!/usr/bin/env python3
"""Golden vectors for weight shift and ballast, worked by hand from the
handbook proportions: CG change = weight moved x distance / total weight;
weight to move = total x (target - CG) / distance; ballast = total x
(target - CG) / (ballast arm - target)."""
import json
import sys
from pathlib import Path

SRC = "Weight-shift and ballast proportions worked in Python (tools/vectors/gen_shift.py)"


def vec(i, inp, exp, src=SRC, tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": t}


def shift():
    out = []
    for w, cg, fr, to, m, tgt in [(2250, 84.3, 150, 90, 50, None), (2250, 84.3, 90, 150, 20, None), (7800, 81.5, 150, 30, None, 80.5),
                                  (3100, 92.0, 140, 70, None, 90.0), (1650, 38.2, 20, 48, 30, None)]:
        d = to - fr
        inp = {"total_weight": f"{w} lb", "cg": f"{cg} in", "from_arm": f"{fr} in", "to_arm": f"{to} in"}
        if m is not None:
            inp["weight"] = f"{m} lb"
            new = cg + m * d / w
        else:
            inp["target_cg"] = f"{tgt} in"
            m = w * (tgt - cg) / d
            new = tgt
        out.append(vec(len(out) + 1, inp, {"result.new_cg.value": new, "result.weight.value": m, "result.cg_change.value": new - cg}))
    base = {"total_weight": "2250 lb", "cg": "84.3 in", "from_arm": "150 in", "to_arm": "90 in"}
    out.append(vec(len(out) + 1, dict(base, target_cg="86 in"), {"ok": False, "error.code": "NO_SOLUTION"}))
    out.append(vec(len(out) + 1, dict(base, to_arm="150 in", weight="10 lb"), {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def ballast():
    out = []
    for w, cg, tgt, arm in [(2250, 84.3, 83.0, 10), (1850, 41.5, 40.0, -20), (3000, 95.0, 97.0, 180), (7800, 81.5, 80.5, 30)]:
        b = w * (tgt - cg) / (arm - tgt)
        out.append(vec(len(out) + 1, {"total_weight": f"{w} lb", "cg": f"{cg} in", "target_cg": f"{tgt} in", "ballast_arm": f"{arm} in"},
                       {"result.ballast.value": b, "result.new_weight.value": w + b}))
    out.append(vec(len(out) + 1, {"total_weight": "2250 lb", "cg": "84.3 in", "target_cg": "83 in", "ballast_arm": "150 in"}, {"ok": False, "error.code": "NO_SOLUTION"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, vs in [("aviation.loading.weight-shift", shift()), ("aviation.loading.ballast", ballast())]:
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
