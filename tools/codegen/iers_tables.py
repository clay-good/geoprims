#!/usr/bin/env python3
"""Regenerates core/crates/gp-astro/src/iers_tables.rs from the IERS
Conventions (2010) electronic tables kept in core/crates/gp-astro/data/iers/:
Tables 5.3a and 5.3b (the IAU 2000A_R06 nutation in longitude and in
obliquity, j = 0 and j = 1 series) and Table 5.2e (the non-polynomial part of Greenwich sidereal time, the
complementary terms of the equation of the equinoxes). Each row keeps its
14 argument multipliers (l, l', F, D, Om, L_Me ... L_Ne, p_A) and its sine
and cosine coefficients in microarcseconds. Standard library only."""
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
DATA = HERE.parent.parent / "core" / "crates" / "gp-astro" / "data" / "iers"


def series(path):
    """{j: [(coefficients, multipliers)]} from an IERS table file."""
    out, j = {}, None
    for line in path.read_text().splitlines():
        s = line.strip()
        if s.startswith("j ="):
            j = int(s.split("=")[1].split()[0])
            out[j] = []
            continue
        f = s.split()
        if j is None or len(f) != 17 or not f[0].isdigit():
            continue
        out[j].append((float(f[1]), float(f[2]), [int(x) for x in f[3:17]]))
    return out


def emit(name, rows):
    lines = [f"pub const {name}: &[([i8; 14], f64, f64)] = &["]
    for sin, cos, m in rows:
        lines.append(f"    ([{', '.join(str(x) for x in m)}], {sin!r}, {cos!r}),")
    lines.append("];")
    return lines


nut = series(DATA / "tab5.3a.txt")
obl = series(DATA / "tab5.3b.txt")
ct = series(DATA / "tab5.2e.txt")
assert len(nut[0]) == 1320 and len(nut[1]) == 38, {k: len(v) for k, v in nut.items()}
assert len(obl[0]) == 1037 and len(obl[1]) == 19, {k: len(v) for k, v in obl.items()}
assert len(ct[0]) == 33 and len(ct[1]) == 1, {k: len(v) for k, v in ct.items()}
out = [
    "//! IERS Conventions (2010) Tables 5.3a and 5.3b (nutation in longitude and in",
    "//! obliquity, IAU 2000A_R06) and 5.2e (non-polynomial part of Greenwich",
    "//! sidereal time), generated from data/iers/tab5.3a.txt, tab5.3b.txt, and",
    "//! tab5.2e.txt by tools/codegen/iers_tables.py.",
    "//! Do not edit by hand. Rows: multipliers of l, l', F, D, Om, L_Me, L_Ve, L_E,",
    "//! L_Ma, L_J, L_Sa, L_U, L_Ne, p_A; then the sine and cosine coefficients in",
    "//! microarcseconds.",
    "",
    "#![allow(clippy::approx_constant, clippy::unreadable_literal, clippy::excessive_precision)]",
    "",
]
out += emit("DPSI_0", nut[0]) + [""] + emit("DPSI_1", nut[1]) + [""] + emit("DEPS_0", obl[0]) + [""] + emit("DEPS_1", obl[1])
out += [""] + emit("EECT_0", ct[0]) + [""] + emit("EECT_1", ct[1])
dest = Path(sys.argv[1] if len(sys.argv) > 1 else HERE.parent.parent / "core" / "crates" / "gp-astro" / "src" / "iers_tables.rs")
dest.write_text("\n".join(out) + "\n")
print(f"wrote {dest}: {len(nut[0])} + {len(nut[1])} longitude rows, {len(obl[0])} + {len(obl[1])} obliquity rows, {len(ct[0])} + {len(ct[1])} complementary rows")
