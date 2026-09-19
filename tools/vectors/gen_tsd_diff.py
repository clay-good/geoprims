#!/usr/bin/env python3
"""Differential fixture for navigation.route.time-speed-distance from Bowditch's
printed Table 11, Speed, Time, and Distance (NGA Pub. 9, Volume II, 2024 edition).

  curl -o bowditch2.pdf "https://msi.nga.mil/api/publications/download?type=view&key=16693975/SFH00000/Bowditch_Vol_2.pdf"
  pdftotext -layout bowditch2.pdf bowditch2.txt
  python tools/vectors/gen_tsd_diff.py bowditch2.txt

Writes core/crates/gp-navigation/tests/data/tsd_table11.csv: every printed cell
as minutes,knots,miles. The table runs 1 to 60 minutes against 0.5 to 40.0 knots
in half-knot steps, printed to a tenth of a mile.

Two notes on reading the PDF:
  - the text layer writes "11. 0" for 11.0, and once "11..0"; both are normalized
  - one cell disagrees with speed x time by more than the tenth it is printed to:
    38 minutes at 10.5 knots prints 6.8 where the arithmetic gives 6.65. Every
    neighbor in that row follows speed x time, so this is a misprint. It is kept
    in the file, flagged, so the parity test pins it instead of silently passing.
"""
import re
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "core/crates/gp-navigation/tests/data/tsd_table11.csv"
EDITION = "NGA Pub. 9 (Bowditch), Volume II, 2024 edition, Table 11"


def cells(text):
    """Every (minutes, knots) -> miles cell of Table 11."""
    lines = text.splitlines()
    out = {}
    for i, line in enumerate(lines):
        if "Speed in knots" not in line:
            continue
        speeds = [float(x) for x in re.findall(r"\d+\.\d", lines[i + 1].replace("utes", " "))]
        for row in lines[i + 3:]:
            head = re.match(r"\s*(\d+)\s", row)
            # "6. 3" is 6.3; one cell in the text layer reads "11..0".
            values = re.findall(r"(\d+)\.+ ?(\d)", row)
            if not head or len(values) != len(speeds):
                continue
            minutes = int(head.group(1))
            for speed, (whole, tenth) in zip(speeds, values):
                out[(minutes, speed)] = float(f"{whole}.{tenth}")
            if minutes == 60:
                break
    return out


def main():
    src = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/gp-src/bowditch2.txt")
    table = cells(src.read_text())
    if len(table) != 60 * 80:
        raise SystemExit(f"parsed {len(table)} cells, expected {60 * 80}")
    misprints = [k for k, v in table.items() if abs(v - k[1] * k[0] / 60) > 0.051]
    rows = [f"# {EDITION}; minutes,knots,miles; misprint marks the cells that differ from speed x time by more than the printed tenth"]
    for (minutes, speed), miles in sorted(table.items()):
        rows.append(f"{minutes},{speed},{miles}" + (",misprint" if (minutes, speed) in misprints else ""))
    OUT.write_text("\n".join(rows) + "\n")
    print(len(table), "cells ->", OUT.name, f"({len(misprints)} misprint: {misprints})")


if __name__ == "__main__":
    main()
