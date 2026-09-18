#!/usr/bin/env python3
"""Regenerates core/crates/gp-time/src/spa_tables.rs from pvlib's NREL SPA
tables (pvlib/spa.py, BSD-3-Clause). Needs pvlib: run it from a scratch
virtualenv (`python3 -m venv v && v/bin/pip install pvlib`)."""
import sys
from pathlib import Path

import pvlib
from pvlib import spa

out = ["//! NREL SPA periodic-term tables (Reda and Andreas 2008, Tables A4.2 and",
       f"//! A4.3), generated from pvlib {pvlib.__version__} `pvlib/spa.py` (BSD-3-Clause) by",
       "//! tools/codegen/spa_tables.py. Do not edit by hand.", "",
       "#![allow(clippy::approx_constant, clippy::excessive_precision, clippy::unreadable_literal)]", ""]


def table(name, arr, width):
    out.append(f"pub const {name}: &[[f64; {width}]] = &[")
    out.extend("    [" + ", ".join(repr(float(v)) for v in r) + "]," for r in arr)
    out.append("];")


for n in ["L0", "L1", "L2", "L3", "L4", "L5", "B0", "B1", "R0", "R1", "R2", "R3", "R4"]:
    table(n, getattr(spa, n), 3)
table("NUTATION_ABCD", spa.NUTATION_ABCD_ARRAY, 4)
table("NUTATION_Y", spa.NUTATION_YTERM_ARRAY, 5)
dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/crates/gp-time/src/spa_tables.rs")
dest.write_text("\n".join(out) + "\n")
