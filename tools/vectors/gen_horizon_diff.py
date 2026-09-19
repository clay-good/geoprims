#!/usr/bin/env python3
"""Differential fixture for navigation.los.horizon from Bowditch's printed
Table 12, Distance of the Horizon (NGA Pub. 9, Volume II, 2024 edition).

  curl -o bowditch2.pdf "https://msi.nga.mil/api/publications/download?type=view&key=16693975/SFH00000/Bowditch_Vol_2.pdf"
  pdftotext -layout bowditch2.pdf bowditch2.txt
  python tools/vectors/gen_horizon_diff.py bowditch2.txt

Writes core/crates/gp-navigation/tests/data/horizon_table12.csv: every printed
row as feet,nautical_miles,statute_miles,meters. The table runs 1 to 820 feet
of height of eye and is the navigator's rule D = 1.17 sqrt(h), with the statute
column computed from the unrounded value (1.17 x 1.15077945 sqrt(h)) rather
than converted from the rounded nautical one.

One cell disagrees with that rule by more than the tenth it is printed to: 640
feet prints 29.5 nautical miles where 1.17 sqrt(640) is 29.599, and the same
row's statute cell (34.1) follows the rule exactly. It is kept in the file,
flagged, so the parity test pins it instead of silently passing.
"""
import math
import re
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "core/crates/gp-navigation/tests/data/horizon_table12.csv"
EDITION = "NGA Pub. 9 (Bowditch), Volume II, 2024 edition, Table 12"
NM_PER_SQRT_FT = 1.17
SM_PER_SQRT_FT = 1.17 * 1.15077945


def rows(text):
    """Every printed (feet) -> (nautical miles, statute miles, meters) row."""
    lines = text.splitlines()
    start = next(i for i, l in enumerate(lines) if l.strip().startswith("Distance of the Horizon"))
    out = {}
    for line in lines[start:]:
        if "TABLE 13" in line or "Geographic Range" in line:
            break
        # Each printed line carries two half-pages: height, nm, statute, meters.
        for feet, nm, statute, meters in re.findall(r"(?<!\.)\b(\d{1,4})\s+(\d+\.\d)\s+(\d+\.\d)\s+(\d*\.\d\d?)\b", line):
            out[int(feet)] = (float(nm), float(statute), float(meters))
    return out


def main():
    src = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/gp-src/bowditch2.txt")
    table = rows(src.read_text())
    if len(table) != 126:
        raise SystemExit(f"parsed {len(table)} heights, expected 126")
    misprints = [ft for ft, (nm, _, _) in table.items() if abs(nm - NM_PER_SQRT_FT * math.sqrt(ft)) > 0.051]
    lines = [f"# {EDITION}; feet,nautical_miles,statute_miles,meters; misprint marks cells that differ from 1.17 sqrt(h) by more than the printed tenth"]
    for feet, (nm, statute, meters) in sorted(table.items()):
        lines.append(f"{feet},{nm},{statute},{meters}" + (",misprint" if feet in misprints else ""))
    OUT.write_text("\n".join(lines) + "\n")
    print(len(table), "heights ->", OUT.name, f"({len(misprints)} misprint: {misprints})")


if __name__ == "__main__":
    main()
