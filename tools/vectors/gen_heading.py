#!/usr/bin/env python3
"""Golden vectors for aviation.wind.heading-chain and
aviation.atmosphere.cloud-base, by hand: TH = TC + WCA, MH = TH - variation
(east positive), CH = MH - deviation read linearly from the card around the
circle; cloud base by 400 ft per degree C and by Bolton's (1980) eq. 15 for
the lifting condensation level, and the freezing level at a lapse rate."""
import json
import math
import sys
from pathlib import Path

SRC = "Heading arithmetic and Bolton (1980) eq. 15 worked in Python (tools/vectors/gen_heading.py)"
SPEC = "add-aviation-suite scenarios"


def n360(a):
    return a % 360.0


def card_dev(card, h):
    card = sorted(card)
    if not card:
        return 0.0
    if len(card) == 1:
        return card[0][1]
    k = max((i for i, (c, _) in enumerate(card) if c <= h), default=len(card) - 1)
    a, b = card[k], card[(k + 1) % len(card)]
    span = (b[0] - a[0]) % 360.0
    return a[1] + ((h - a[0]) % 360.0) / span * (b[1] - a[1])


def vec(i, inp, exp, src=SRC, tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": t}


def chain():
    out = []
    card = [(30, 1), (60, 2), (90, -1), (120, -2), (180, 0), (270, 1.5), (330, 0.5)]
    cases = [(70, -5, -10, card[:4]), (70, -5, -10, card), (180, 8, 12, card), (350, 12, -3, card), (0, 0, 0, []), (255, -3, 7.5, card), (20, 0, 14, card)]
    for tc, wca, var, cd in cases:
        th = n360(tc + wca)
        mh = n360(th - var)
        dev = card_dev(cd, mh)
        ch = n360(mh - dev)
        inp = {"true_course": f"{tc} deg", "wind_correction": f"{wca} deg", "variation": f"{var} deg"}
        if cd:
            inp["deviation_card"] = [{"heading": f"{h} deg", "deviation": f"{d} deg"} for h, d in cd]
        out.append(vec(len(out) + 1, inp, {"result.true_heading.value": th, "result.magnetic_heading.value": mh, "result.deviation.value": dev, "result.compass_heading.value": ch},
                       SPEC if len(out) == 0 else SRC))
    out.append(vec(len(out) + 1, {"true_course": "70 deg", "variation": "0 deg", "deviation_card": [{"heading": "60 deg", "deviation": "40 deg"}]}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def cloud():
    out = []
    for t, td, elev, lapse in [(25, 15, None, None), (30, 10, 5430, None), (15, 14, 0, None), (8, 2, 2000, 3.0), (35, 5, 1000, None), (-5, -9, 6000, None)]:
        tk, tdk = t + 273.15, td + 273.15
        tl = 1 / (1 / (tdk - 56) + math.log(tk / tdk) / 800) + 56
        lcl_m = (tk - tl) / (9.80665 / 1004.0)
        inp = {"temperature": f"{t} degC", "dew_point": f"{td} degC"}
        exp = {"result.cloud_base_rule.value": (t - td) * 400.0, "result.cloud_base_lcl.value": lcl_m / 0.3048}
        if elev is not None:
            inp["elevation"] = f"{elev} ft"
            exp["result.cloud_base_msl.value"] = lcl_m / 0.3048 + elev
        if lapse is not None:
            inp["lapse_rate"] = lapse
        if t > 0:
            exp["result.freezing_level.value"] = t / (lapse or 1.98) * 1000 + (elev or 0)
        out.append(vec(len(out) + 1, inp, exp, SPEC if len(out) == 0 else SRC, tol=1e-6))
    out.append(vec(len(out) + 1, {"temperature": "10 degC", "dew_point": "12 degC"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, vs in [("aviation.wind.heading-chain", chain()), ("aviation.atmosphere.cloud-base", cloud())]:
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
