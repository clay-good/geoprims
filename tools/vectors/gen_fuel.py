#!/usr/bin/env python3
"""Golden vectors for fuel planning, worked by hand: trip = sum of time x
burn, alternate and reserve = time x cruise burn (the last leg's burn unless
given), total = taxi + climb + trip + alternate + reserve. Reserve minutes
from 14 CFR 91.151 and 91.167: airplane VFR day 30, night 45; rotorcraft
VFR 20; IFR 45, helicopter IFR 30."""
import json
import sys
from pathlib import Path

SRC = "Fuel arithmetic worked in Python (tools/vectors/gen_fuel.py)"
SPEC = "add-aviation-suite fuel scenarios"
L_PER_GAL = 3.785411784
RESERVE = {("vfr-day", "airplane"): 30, ("vfr-night", "airplane"): 45, ("vfr-day", "rotorcraft"): 20,
           ("vfr-night", "rotorcraft"): 20, ("ifr", "airplane"): 45, ("ifr", "rotorcraft"): 30}


def vec(i, inp, exp, src=SRC, tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": t}


def plan(legs, reserve, category="airplane", taxi=0, climb=0, alt_min=0, cruise=None, custom=None, usable=None):
    trip = sum(t * b for t, b in legs)
    cruise = cruise if cruise is not None else legs[-1][1]
    rmin = custom if reserve == "custom" else 0 if reserve == "none" else RESERVE[(reserve, category)]
    alt = alt_min / 60 * cruise
    res = rmin / 60 * cruise
    total = taxi + climb + trip + alt + res
    exp = {"result.total.value": total, "result.trip.value": trip, "result.reserve_fuel.value": res, "result.reserve_time.value": rmin}
    if alt_min:
        exp["result.alternate_fuel.value"] = alt
    if usable is not None:
        exp["result.margin.value"] = usable - total
        exp["result.endurance.value"] = max(usable - taxi, 0) / cruise
    for k, (t, b) in enumerate(legs):
        exp[f"result.legs.{k}.fuel.value"] = t * b
    return exp


def fuel():
    out = []
    L = lambda legs: [{"time": f"{t} h", "burn": f"{b} gph"} for t, b in legs]
    legs = [(1.5, 9.5), (0.75, 9)]
    out.append(vec(1, {"legs": L(legs), "reserve": "vfr-night", "taxi": "1.4 gal", "usable_fuel": "53 gal"},
                   dict(plan(legs, "vfr-night", taxi=1.4, usable=53), **{"result.reserve_rule": "14 CFR 91.151(a)(2) minimum, rules as of 2026-09-22; operators may require more. Not legal advice: https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-91/subpart-C/section-91.151"}),
                   SPEC + " (VFR night: 45 minutes at the cruise burn, cited to 14 CFR 91.151(a)(2))"))
    heli = [(1.2, 25)]
    out.append(vec(2, {"legs": L(heli), "reserve": "vfr-day", "category": "rotorcraft"}, plan(heli, "vfr-day", "rotorcraft"),
                   SPEC + " (rotorcraft VFR: 20 minutes, 14 CFR 91.151(b))"))
    ifr = [(2, 12)]
    out.append(vec(3, {"legs": L(ifr), "reserve": "ifr", "alternate_time": "40 min", "cruise_burn": "11 gph", "climb": "2 gal", "usable_fuel": "40 gal"},
                   plan(ifr, "ifr", climb=2, alt_min=40, cruise=11, usable=40)))
    out.append(vec(4, {"legs": L(ifr), "reserve": "ifr", "category": "rotorcraft", "alternate_time": "20 min"}, plan(ifr, "ifr", "rotorcraft", alt_min=20)))
    day = [(0.5, 8), (1, 8.5), (0.25, 7)]
    out.append(vec(5, {"legs": L(day), "reserve": "vfr-day", "usable_fuel": "10 gal"}, plan(day, "vfr-day", usable=10)))
    out.append(vec(6, {"legs": L(day), "reserve": "custom", "custom_reserve": "60 min", "cruise_burn": "8 gph"}, plan(day, "custom", cruise=8, custom=60)))
    out.append(vec(7, {"legs": L([(1, 9)]), "reserve": "none"}, plan([(1, 9)], "none")))
    # Metric: 40 L/h for 2 h is 80 L, about 21.1 US gal.
    out.append(vec(8, {"legs": [{"time": "2 h", "burn": "40 L/h"}], "reserve": "vfr-day"}, plan([(2, 40 / L_PER_GAL)], "vfr-day")))
    for inp in [{"legs": L(day), "reserve": "custom"},
                {"legs": L(day), "reserve": "vfr-day", "taxi": "-1 gal"},
                {"legs": [{"time": "1 h", "burn": "0 gph"}], "reserve": "vfr-day"},
                {"legs": L(day), "reserve": "vfr-day", "custom_reserve": "60 min"}]:
        out.append(vec(len(out) + 1, inp, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "aviation.loading.fuel-plan.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in fuel()))


if __name__ == "__main__":
    main()
