#!/usr/bin/env python3
"""Appends drone.power.endurance vectors written from the published formulas.

    python3 tools/vectors/gen_endurance.py

Append-only: v001-v009 (gen_drone.py) are published and are left alone, and a
vector whose id is already in the file is skipped, so a rerun changes nothing.

The model, from Bauersfeld and Scaramuzza (2022), Sec. VII-E step 6, plus the
reserve, derating and range the tool's spec adds:

    usable  = E × usable share × (1 − derating)
    flyable = usable × (1 − reserve)
    t       = flyable / P,   range = t_cruise (or t_hover) × groundspeed
    cold derating (heuristic) = min(0.5, max(0, (20 − T°C) × 1%))
"""
import json
from pathlib import Path

FILE = Path(__file__).resolve().parents[2] / "core/vectors/drone.power.endurance.jsonl"
SRC = "Bauersfeld and Scaramuzza (2022) Sec. VII-E step 6, t = E / P, with the spec's reserve, derating, and range, evaluated in Python (tools/vectors/gen_endurance.py)"
VER = "IEEE RA-L 7(2), 2022"
PAPER = "Bauersfeld and Scaramuzza, Range, Endurance, and Optimal Speed Estimates for Multicopters, IEEE RA-L 7(2) 2022, Sec. VII-E step 6 (DJI Mavic 3: C_eff 4.89 Ah and 4.88 Ah, 4S at 3.7 V per cell, 89.5 W and 107.0 W give 2909 s and 2429 s)"
TIGHT = {"rel": 1e-9, "abs": 1e-9}


def cold(t_c):
    return 0.0 if t_c >= 20 else min(0.5, (20 - t_c) * 0.01)


def model(e, p, usable=100, reserve=0, derate=0.0, cruise=None, gs=None):
    avail = e * usable / 100 * (1 - derate)
    fly = avail * (1 - reserve / 100)
    out = {
        "result.hover_time.value": fly / p * 60,
        "result.hover_time_no_reserve.value": avail / p * 60,
        "result.usable_energy.value": avail,
        "result.reserve_energy.value": avail * reserve / 100,
        "result.derating_applied": derate * 100,
    }
    if cruise:
        out["result.cruise_time.value"] = fly / cruise * 60
    if gs:
        out["result.range.value"] = fly / (cruise or p) * 3600 * gs / 1000
    return out


def vec(inp, expect, source=SRC, version=VER, tol=None):
    return {"input": inp, "expect": {**expect, "ok": True} if expect.get("ok") is not False else expect,
            "source": source, "sourceVersion": version,
            "tolerance": tol if tol is not None else {k: TIGHT for k, v in expect.items() if isinstance(v, float)}}


def err(inp, field):
    return vec(inp, {"ok": False, "error.code": "INVALID_INPUT", "error.field": field}, tol={})


def vectors():
    out = []
    # The published worked example. C_eff is printed to 3 significant figures
    # (±0.005 Ah, ±3 s), so the tolerance is 0.06 min (3.6 s) around the
    # printed 2909 s and 2429 s, not the tight one.
    out.append(vec({"energy": f"{round(4.89 * 3.7 * 4, 6)} Wh", "power": "89.5 W"},
                   {"result.hover_time.value": 2909 / 60}, PAPER, VER,
                   {"result.hover_time.value": {"abs": 0.06, "rel": 0}}))
    out.append(vec({"energy": f"{round(4.88 * 3.7 * 4, 6)} Wh", "power": "89.5 W", "cruise_power": "107 W"},
                   {"result.cruise_time.value": 2429 / 60}, PAPER, VER,
                   {"result.cruise_time.value": {"abs": 0.06, "rel": 0}}))
    # Range from the same operating point: t_r × v_r at 13.12 m/s. The paper
    # prints 32.1 km, but its own t_r × v_r is 31.9 km; this is the formula.
    e_r = round(4.88 * 3.7 * 4, 6)
    out.append(vec({"energy": f"{e_r} Wh", "power": "89.5 W", "cruise_power": "107 W", "groundspeed": "13.12 m/s"},
                   {k: v for k, v in model(e_r, 89.5, cruise=107, gs=13.12).items() if k in ("result.range.value", "result.cruise_time.value", "result.hover_time.value")}))
    # Reserve and usable share: both outputs, and the energy split.
    for e, u, p, r in [(90.4, 80, 150.6, 20), (77, 100, 180, 30), (5870 / 1000 * 15.4, 90, 250, 25)]:
        m = model(e, p, u, r)
        out.append(vec({"energy": f"{round(e, 6)} Wh", "usable": u, "power": f"{p} W", "reserve": r}, m))
    # The heuristic cold derating: none at 20 °C and warmer, 1% per °C below, capped at 50%.
    for t in [25, 20, 10, 0, -10, -40]:
        d = cold(t)
        m = model(100, 100, derate=d)
        exp = {k: m[k] for k in ("result.hover_time.value", "result.usable_energy.value", "result.derating_applied")}
        if d > 0:
            exp["meta.warnings.*.code"] = "HEURISTIC_DERATING"
        out.append(vec({"energy": "100 Wh", "power": "100 W", "battery_temperature": f"{t} degC"}, exp))
    # A derating the user enters overrides the temperature heuristic.
    m = model(90.4, 150.6, 80, 20, derate=0.15)
    out.append(vec({"energy": "90.4 Wh", "usable": 80, "power": "150.6 W", "reserve": 20, "derating": 15, "battery_temperature": "0 degC"},
                   {k: m[k] for k in ("result.hover_time.value", "result.usable_energy.value", "result.derating_applied")}))
    # Range at hover power when no cruise power is given; units converted on the way in.
    m = model(500, 1200, 85, 20, gs=15)
    out.append(vec({"energy": "500 Wh", "usable": 85, "power": "1.2 kW", "reserve": 20, "groundspeed": "15 m/s"},
                   {k: m[k] for k in ("result.hover_time.value", "result.range.value")}))
    # Energy in kilojoules: 360 kJ is exactly 100 Wh.
    out.append(vec({"energy": "360 kJ", "power": "200 W"}, {"result.hover_time.value": 30.0}))
    # Errors.
    out.append(err({"energy": "0 Wh", "power": "100 W"}, "/energy"))
    out.append(err({"energy": "100 Wh", "power": "-5 W"}, "/power"))
    out.append(err({"energy": "100 Wh", "power": "100 W", "cruise_power": "0 W"}, "/cruise_power"))
    out.append(err({"energy": "100 Wh", "power": "100 W", "reserve": 100}, "/reserve"))
    out.append(err({"energy": "100 Wh", "power": "100 W", "derating": 96}, "/derating"))
    return out


def main():
    lines = [line for line in FILE.read_text().splitlines() if line.strip()]
    have = [json.loads(line) for line in lines]
    ids = {v["id"] for v in have}
    n = 9  # gen_drone.py wrote v001-v009; these follow in a fixed order
    new = []
    for v in vectors():
        n += 1
        vid = f"v{n:03d}"
        if vid in ids:
            continue
        new.append(json.dumps({"id": vid, **v}, ensure_ascii=False, separators=(",", ":")))
    if new:
        FILE.write_text("\n".join(lines + new) + "\n")
    print(f"appended {len(new)}")


if __name__ == "__main__":
    main()
