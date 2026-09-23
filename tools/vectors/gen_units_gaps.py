#!/usr/bin/env python3
"""Golden vectors for the units a converter offers but no vector ever used.

`gen_units.py` sweeps every ordered pair of the units in its own tables, but
those tables were never the whole enum: fifteen units across seven quantities
are offered by the core and were reachable by no vector at all. A unit with no
vector is a definition nobody ever checked -- a survey acre that is really an
international acre, a milliarcsecond off by a factor of a thousand, a per-year
rate built on a calendar year instead of a Julian one -- and four of the seven
converters were already past the stable bar with the hole in them.

This sweeps every ordered pair with at least one of those units on a side.
The expected values are computed the same way `gen_units.py` computes its own,
from the published exact definition in rational arithmetic, rounded to binary64
once at the end; the definitions restated below are the ones the core carries.

The pairs already covered are left alone, so `gen_units.py` still reproduces
its own output byte for byte. Published vectors are frozen, so this APPENDS.
Run it once.

    python3 tools/vectors/gen_units_gaps.py
"""
import json
import sys
from fractions import Fraction as F
from pathlib import Path

from gen_units import LINEAR, PI_UNITS, REL_EXACT, REL_PI, SRC, SWEEP_VALUES, VER

# Like gen_units.py, an output directory may be given so the reproducibility
# gate can regenerate the whole chain into a scratch directory.
ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")

FT = F(3048, 10000)
MI = 5280 * FT
FTUS = F(1200, 3937)
# The Julian year of 365.25 days exactly, which is what a geodetic velocity in
# mm/yr or a clock drift in ppb/yr is quoted against -- not the calendar year.
YR = 31_557_600

# The units the core offers that gen_units.py's tables never listed. Each is
# restated here from its own definition rather than copied from the core.
EXTRA = {
    "length": {"um": F(1, 10**6), "Mm": 10**6},
    "area": {"NM2": 1852**2, "ftUS2": FTUS**2, "acUS": 43560 * FTUS**2},
    "speed": {"mm/yr": F(1, 1000 * YR), "m/yr": F(1, YR)},
    "angle": {"mas": F(1, 3_600_000)},
    "angular-rate": {"arcsec/yr": F(1, 3600 * YR), "mas/yr": F(1, 3_600_000 * YR)},
    "frequency": {"ppm/yr": F(1, 10**6 * YR), "ppb/yr": F(1, 10**9 * YR)},
    "energy": {"ft*lbf": FT * F(45359237, 10**8) * F(980665, 10**5)},
}


def convert(group, x, a, b):
    """The conversion in exact rational arithmetic, rounded to binary64 once."""
    table = {**LINEAR.get(group, {}), **EXTRA[group]}
    if a in table and b in table:
        return float(F(x) * F(table[a]) / F(table[b])), REL_EXACT
    # A pi unit on one side: the ratio is irrational and carries the looser bound.
    fa = PI_UNITS.get(group, {}).get(a) or float(table[a])
    fb = PI_UNITS.get(group, {}).get(b) or float(table[b])
    return float(F(x)) * fa / fb, REL_PI


def cases(group):
    """Ordered pairs with at least one previously uncovered unit on a side."""
    new = sorted(EXTRA[group])
    units = sorted(set(LINEAR.get(group, {})) | set(PI_UNITS.get(group, {})) | set(new))
    pairs = [(a, b) for a in units for b in units if a != b and (a in new or b in new)]
    return [(SWEEP_VALUES[k % len(SWEEP_VALUES)], a, b) for k, (a, b) in enumerate(pairs)]


def main():
    total = 0
    for group in EXTRA:
        path = ROOT / f"units.{group}.convert.jsonl"
        existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        start = max(int(v["id"][1:]) for v in existing)
        if start > 22:
            raise SystemExit(f"{path.name} already carries {start} vectors; this script appends once")
        rows = []
        for i, (x, a, b) in enumerate(cases(group), start=start + 1):
            want, rel = convert(group, x, a, b)
            rows.append({
                "id": f"v{i:03d}",
                "input": {"value": f"{x} {a}", "to": b},
                "expect": {"ok": True, "result.converted.value": want, "result.converted.unit": b},
                "source": SRC,
                "sourceVersion": VER,
                "tolerance": {"result.converted.value": {"rel": rel}},
            })
        with path.open("a") as f:
            for r in rows:
                f.write(json.dumps(r) + "\n")
        total += len(rows)
        print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")
    print(f"{total} vectors in all")


if __name__ == "__main__":
    main()
