#!/usr/bin/env python3
"""Golden vectors for the three units tools that are not plain converters.

`units.fuel.convert`, `units.slope.convert` and `units.quantity.normalize` each
carried six hand-picked vectors -- enough to show the tool works, too few to
promote it. This brings each to the twenty the stable bar asks for, by sweeping
the cases the hand-picked six do not reach.

Each expected value is computed here from the published definitions, not from
the core:

- fuel: mass = volume x density, done in SI from the exact definitions of the
  gallon, the pound and the litre, then converted to pounds. The core works in
  the density's own units instead, so 50 gal at 6 lb/gal comes out exactly 300
  lb; these two routes agree to a bit or two, which is what the tolerance
  allows and what makes the comparison worth making.
- slope: ratio = tan(angle), percent = 100 x ratio, per mille = 1000 x ratio,
  from Python's math, which is a different implementation of tan and atan than
  the core's libm.
- normalize: the canonical value of each quantity, in exact rational
  arithmetic from the same definitions gen_units.py uses.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_units_special.py
"""
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

from gen_units import REL_EXACT, SRC, VER

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")

LB = F(45359237, 10**8)          # kg, exactly
GAL = F(3785411784, 10**12)      # m3, exactly
FT = F(3048, 10000)
L = F(1, 1000)
G0 = F(980665, 100000)

VOL = {"galUS": GAL, "L": L, "m3": F(1), "ft3": FT**3}
# Densities in kg/m3, from their own definitions.
DENS = {"6 lb/galUS": 6 * LB / GAL, "6.7 lb/galUS": F("6.7") * LB / GAL,
        "6.02 lb/galUS": F("6.02") * LB / GAL, "0.8 g/cm3": F(800), "800 kg/m3": F(800)}
FUEL_DENSITY = {"avgas-100ll": "6 lb/galUS", "jet-a": "6.7 lb/galUS"}

# (volume, unit, how the density is given)
FUEL_TO_MASS = [
    ("26.2", "galUS", "avgas-100ll"), ("200", "L", "jet-a"), ("1", "ft3", "avgas-100ll"),
    ("75.5", "galUS", "jet-a"), ("0.5", "m3", "0.8 g/cm3"), ("40", "galUS", "6.02 lb/galUS"),
    ("1000", "L", "800 kg/m3"), ("12.5", "galUS", "avgas-100ll"),
]
# (mass, unit, how the density is given)
FUEL_TO_VOLUME = [
    ("157.2", "lb", "avgas-100ll"), ("1340", "lb", "jet-a"), ("500", "kg", "0.8 g/cm3"),
    ("240.8", "lb", "6.02 lb/galUS"), ("2000", "lb", "jet-a"), ("100", "kg", "800 kg/m3"),
]

# (value, the representation it is given in)
SLOPE = [
    ("1", "percent"), ("3", "percent"), ("15", "percent"), ("-2.5", "percent"),
    ("100", "percent"), ("0", "percent"),
    ("0.02", "ratio"), ("0.25", "ratio"), ("-0.125", "ratio"), ("2", "ratio"),
    ("10", "permille"), ("125", "permille"), ("-40", "permille"),
    ("1", "degrees"), ("30", "degrees"), ("-15", "degrees"), ("60", "degrees"),
    ("89", "degrees"), ("0.5", "degrees"),
]

# (text, quantity, canonical value, canonical unit)
NORMALIZE = [
    ("2550 lb", "mass", float(2550 * LB), "kg"),
    ("1 t", "mass", 1000.0, "kg"),
    ("40 ac", "area", float(40 * 43560 * FT**2), "m2"),
    ("50 galUS", "volume", float(50 * GAL), "m3"),
    ("100 kt", "speed", float(100 * F(1852, 3600)), "m/s"),
    ("500 ft/min", "vertical-speed", float(500 * FT / 60), "m/s"),
    ("2 g0", "acceleration", float(2 * G0), "m/s2"),
    ("100 gon", "angle", 90.0, "deg"),
    ("3 rpm", "angular-rate", 18.0, "deg/s"),
    ("90 min", "time", 5400.0, "s"),
    ("1 kWh", "energy", 3_600_000.0, "J"),
    ("180 hp", "power", float(180 * 550 * FT * LB * G0), "W"),
    ("5000 mAh", "electric-charge", 18_000.0, "C"),
    ("2.4 GHz", "frequency", 2.4e9, "Hz"),
    ("20 Mbit/s", "data-rate", 2e7, "bit/s"),
    ("6.7 lb/galUS", "density", float(F("6.7") * LB / GAL), "kg/m3"),
    ("18 degF", "temperature-difference", 10.0, "K"),
    ("1013.25 hPa", "pressure", 101325.0, "Pa"),
]

FUEL_SRC = ("FAA-H-8083-25C Chapter 10 nominal fuel weights (avgas 6 lb/gal, jet fuel 6.7 lb/gal); "
            "mass = volume x density in SI from the exact gallon, litre and pound, by rational arithmetic")
SLOPE_SRC = "Definition of grade: ratio = rise/run = tan(angle); angles from Python math.tan and math.atan"


def fuel_rows(start):
    rows = []
    i = start
    for x, unit, spec in FUEL_TO_MASS:
        i += 1
        sym = FUEL_DENSITY.get(spec, spec)
        kg = F(x) * VOL[unit] * DENS[sym]
        key = "fuel" if spec in FUEL_DENSITY else "density"
        rows.append((i, {"volume": f"{x} {unit}", key: spec},
                     {"ok": True, "result.mass.value": float(kg / LB), "result.mass.unit": "lb"},
                     {"result.mass.value": {"rel": REL_EXACT}}, FUEL_SRC))
    for x, unit, spec in FUEL_TO_VOLUME:
        i += 1
        sym = FUEL_DENSITY.get(spec, spec)
        kg = F(x) * (LB if unit == "lb" else F(1))
        m3 = kg / DENS[sym]
        key = "fuel" if spec in FUEL_DENSITY else "density"
        rows.append((i, {"mass": f"{x} {unit}", key: spec},
                     {"ok": True, "result.volume.value": float(m3 / GAL), "result.volume.unit": "galUS"},
                     {"result.volume.value": {"rel": REL_EXACT}}, FUEL_SRC))
    return rows


def slope_rows(start):
    rows = []
    for k, (x, frm) in enumerate(SLOPE, start=start + 1):
        v = float(x)
        if frm == "degrees":
            ratio, deg = math.tan(math.radians(v)), v
        else:
            ratio = v / {"ratio": 1.0, "percent": 100.0, "permille": 1000.0}[frm]
            deg = math.degrees(math.atan(ratio))
        expect = {"ok": True, "result.ratio.value": ratio, "result.percent.value": ratio * 100.0,
                  "result.permille.value": ratio * 1000.0, "result.degrees.value": deg}
        # tan and atan are correctly rounded by neither implementation, so the
        # bound is a few ulp rather than the one rounding an exact ratio costs.
        tol = {k2: {"rel": 2e-15, "abs": 1e-15} for k2 in expect if k2 != "ok"}
        rows.append((k, {"value": v, "from": frm}, expect, tol, SLOPE_SRC))
    return rows


def normalize_rows(start):
    rows = []
    for k, (text, q, want, unit) in enumerate(NORMALIZE, start=start + 1):
        rows.append((k, {"value": text, "quantity": q},
                     {"ok": True, "result.normalized.value": want, "result.normalized.unit": unit},
                     {"result.normalized.value": {"rel": REL_EXACT}}, SRC))
    return rows


def main():
    plan = [
        ("units.fuel.convert", fuel_rows, "FAA-H-8083-25C (2023)"),
        ("units.slope.convert", slope_rows, "definition"),
        ("units.quantity.normalize", normalize_rows, VER),
    ]
    for tool, build, ver in plan:
        path = ROOT / f"{tool}.jsonl"
        existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        start = max(int(v["id"][1:]) for v in existing)
        if start > 6:
            raise SystemExit(f"{path.name} already carries {start} vectors; this script appends once")
        rows = build(start)
        with path.open("a") as f:
            for i, inp, expect, tol, src in rows:
                f.write(json.dumps({
                    "id": f"v{i:03d}", "input": inp, "expect": expect,
                    "source": src, "sourceVersion": ver, "tolerance": tol,
                }) + "\n")
        print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
