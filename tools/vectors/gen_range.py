#!/usr/bin/env python3
"""Golden vectors for specific range, worked by hand: NM per US gallon =
speed / fuel flow, fuel per 100 NM = 100 x flow / speed. A liter-per-hour
flow is converted at 3.785411784 L per US gallon."""
import json
import sys
from pathlib import Path

SRC = "Specific range worked in Python (tools/vectors/gen_range.py)"
FAA = "FAA Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C), chapter 16 fuel-consumption example"
L_PER_GAL = 3.785411784


def vec(i, inp, exp, src=SRC, version="2026", tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": version, "tolerance": t}


def cases():
    # Preserve the eight originally published vectors byte for byte.
    out = []
    for tas, ff, gs, unit in [(120, 8.5, 105, "gph"), (150, 12, None, "gph"), (95, 6, 110, "gph"), (250, 30, 280, "gph"), (140, 40, 130, "L/h")]:
        ffg = ff / L_PER_GAL if unit == "L/h" else ff
        inp = {"tas": f"{tas} kt", "fuel_flow": f"{ff} {unit}"}
        exp = {"result.air_range": tas / ffg, "result.fuel_per_100nm.value": 100 * ffg / tas}
        if gs:
            inp["groundspeed"] = f"{gs} kt"
            exp.update({"result.ground_range": gs / ffg, "result.ground_fuel_per_100nm.value": 100 * ffg / gs})
        out.append(vec(len(out) + 1, inp, exp))
    for inp in [{"tas": "120 kt", "fuel_flow": "0 gph"}, {"tas": "0 kt", "fuel_flow": "8 gph"}, {"tas": "120 kt", "fuel_flow": "8 gph", "groundspeed": "0 kt"}]:
        out.append(vec(len(out) + 1, inp, {"ok": False, "error.code": "INVALID_INPUT"}))
    out.append(vec(9, {"tas": "100 kt", "fuel_flow": "5 gph", "groundspeed": "100 kt"}, {
        "result.air_range": 20.0,
        "result.fuel_per_100nm.value": 5.0,
        "result.ground_range": 20.0,
        "result.ground_fuel_per_100nm.value": 5.0,
    }, FAA, "FAA-H-8083-25C (2023)"))
    for tas, ff, gs in [
        (80, 8, None),
        (100, 5, 80),
        (200, 5, 80),
        (100, 10, 80),
        (160, 8, 120),
        (100, 10, 120),
        (55, 4.5, 40),
        (320, 36, 360),
    ]:
        inp = {"tas": f"{tas} kt", "fuel_flow": f"{ff} gph"}
        exp = {
            "result.air_range": tas / ff,
            "result.fuel_per_100nm.value": 100.0 * ff / tas,
        }
        if gs is not None:
            inp["groundspeed"] = f"{gs} kt"
            exp.update({
                "result.ground_range": gs / ff,
                "result.ground_fuel_per_100nm.value": 100.0 * ff / gs,
            })
        out.append(vec(len(out) + 1, inp, exp))
    for inp in [
        {"tas": "-1 kt", "fuel_flow": "8 gph"},
        {"tas": "120 kt", "fuel_flow": "-1 gph"},
        {"tas": "120 kt", "fuel_flow": "8 gph", "groundspeed": "-1 kt"},
    ]:
        out.append(vec(len(out) + 1, inp, {"ok": False, "error.code": "INVALID_INPUT"}))
    assert len(out) == 20
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "aviation.performance.specific-range.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in cases()))


if __name__ == "__main__":
    main()
