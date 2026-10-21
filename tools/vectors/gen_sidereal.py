#!/usr/bin/env python3
"""Golden vectors for time.scale.sidereal (add-celestial-and-moon,
time/sidereal-and-calendar), from ERFA through pyerfa: era00, gmst06, and
gst06a at random instants from 1972 to 2050, with UTC to TT by utctai and
taitt and UT1 by utcut1. Reads the fixture
core/crates/gp-time/tests/data/erfa_sidereal.json (generated with pyerfa
2.0.1.5) and expects each value within 1e-9 rad, the spec's tolerance:
3.8e-9 hours of sidereal time, 5.7e-8 degrees of rotation angle.

    gen_sidereal.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import json
import math
from pathlib import Path

FIX = Path(__file__).resolve().parents[2] / "core" / "crates" / "gp-time" / "tests" / "data" / "erfa_sidereal.json"
f = json.loads(FIX.read_text())
H = 24 / (2 * math.pi)
rows = [{"input": {"utc": "2000-01-01T12:00:00Z", "dut1": "0 s"},
         "expect": {"ok": True, "result.gmst_meeus": "18h 41m 50.548s", "result.era.value": 280.46061837504},
         "source": "add-celestial-and-moon scenario (J2000.0); ERA by IERS 2010 eq. 5.14 at Tu = 0", "sourceVersion": "IERS Conventions (2010)",
         "tolerance": {"result.era.value": {"abs": 1e-9}}}]
for r in f["rows"][:21]:
    rows.append({"input": {"utc": r["utc"], "dut1": f"{r['dut1']} s"},
                 "expect": {"ok": True, "result.era.value": math.degrees(r["era"]), "result.gmst_decimal": r["gmst"] * H, "result.gast_decimal": r["gast"] * H},
                 "source": f["source"], "sourceVersion": "ERFA 2.0.1",
                 "tolerance": {"result.era.value": {"abs": 5.7e-8}, "result.gmst_decimal": {"abs": 3.8e-9}, "result.gast_decimal": {"abs": 3.8e-9}}})
for i, r in enumerate(rows, 1):
    print(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":")))
