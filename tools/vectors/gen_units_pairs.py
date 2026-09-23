#!/usr/bin/env python3
"""Golden vectors for every ordered unit pair a converter still has not seen.

The derivation notes say the converters are checked against "every ordered
pair" of their units. That was a sample, not a sweep: `gen_units.py` fills each
group up to twenty-two vectors and then stops, so the length converter covered
thirteen of its hundred and thirty-two pairs and the pressure converter
twenty-one of seventy-two. A sample catches a factor that is wrong everywhere.
It does not catch one unit's factor being wrong, which is the mistake that
actually happens, and which only the pair that uses it can find.

This appends the remaining pairs so the claim is literally true. Expected
values are computed the way `gen_units.py` computes its own, from each unit's
published exact definition in rational arithmetic, rounded to binary64 once at
the end.

Published vectors are frozen, so this APPENDS, and it skips any pair a vector
already covers. Run it once.

    python3 tools/vectors/gen_units_pairs.py
"""
import json
import sys
from pathlib import Path

from gen_units import LINEAR, PI_UNITS, SRC, SWEEP_VALUES, VER
from gen_units_gaps import EXTRA, convert

# Like gen_units.py, an output directory may be given so the reproducibility
# gate can regenerate the whole chain into a scratch directory.
ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")


def main():
    total = 0
    for group in LINEAR:
        path = ROOT / f"units.{group}.convert.jsonl"
        existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        covered = {(v["input"]["value"].split(" ", 1)[1], v["input"]["to"]) for v in existing}
        units = sorted(set(LINEAR[group]) | set(PI_UNITS.get(group, {})) | set(EXTRA.get(group, {})))
        missing = [(a, b) for a in units for b in units if a != b and (a, b) not in covered]
        if not missing:
            continue
        start = max(int(v["id"][1:]) for v in existing)
        rows = []
        for k, (a, b) in enumerate(missing):
            x = SWEEP_VALUES[k % len(SWEEP_VALUES)]
            want, rel = convert(group, x, a, b) if group in EXTRA else _plain(group, x, a, b)
            rows.append({
                "id": f"v{start + k + 1:03d}",
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


def _plain(group, x, a, b):
    """A group with no added units converts through gen_units.py's own tables."""
    from gen_units import lin

    return lin(group, x, a, b)


if __name__ == "__main__":
    main()
