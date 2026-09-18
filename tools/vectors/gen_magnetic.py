#!/usr/bin/env python3
"""Golden vectors for the geomagnetism tools (core/vectors/geodesy.magnetic.*.jsonl).

WMM2025 expectations are the NCEI test values (WMM2025_TestValues.txt, shipped
with the coefficients). IGRF-14 expectations come from ppigrf 2.1 (an
independent pure-Python IGRF), evaluated on January 1 of coefficient epochs,
where its calendar-time interpolation and IGRF's decimal-year interpolation
agree exactly. Needs `ppigrf` in a scratch virtualenv. Rerunning must
reproduce the files.
"""
import json
import sys
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TEST_VALUES = ROOT / "core/crates/gp-geodesy/tests/data/WMM2025_TestValues.txt"
WMM_SRC = "NCEI WMM2025 test values (WMM2025_TestValues.txt, released with the WMM2025 coefficients)"
WMM_VER = "WMM2025 (2024-11-13)"
IGRF_SRC = "ppigrf 2.1.0 (independent pure-Python IGRF-14 synthesis), on January 1 of coefficient epochs (tools/vectors/gen_magnetic.py)"
IGRF_VER = "IGRF-14 (2024-12)"


def vec(i, inp, exp, tol, src, ver):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: tol[k] for k in exp}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": t}


def wmm_vectors():
    out = []
    rows = [l.split() for l in TEST_VALUES.read_text().splitlines() if l.strip() and not l.startswith("#")]
    # Every tenth official point keeps the file small; the Rust test runs all 100.
    for i, r in enumerate(rows[::10], 1):
        v = list(map(float, r))
        inp = {"lat": v[2], "lon": v[3], "height": f"{r[1]} km", "date": r[0]}
        exp = {"result.declination.value": v[4], "result.inclination.value": v[5],
               "result.horizontal_intensity": v[6], "result.total_intensity": v[10]}
        tol = {"result.declination.value": {"abs": 0.005}, "result.inclination.value": {"abs": 0.005},
               "result.horizontal_intensity": {"abs": 0.001}, "result.total_intensity": {"abs": 0.001}}
        out.append(vec(i, inp, exp, tol, WMM_SRC, WMM_VER))
    return out


def igrf_vectors(start):
    import numpy as np
    import ppigrf
    cases = [(40.015, -105.27, 1.655, 1985), (51.5, -0.13, 0.0, 1950), (-33.9, 18.4, 0.0, 2000),
             (64.8, -147.7, 0.2, 2015), (-77.85, 166.67, 0.0, 1970), (35.7, 139.7, 0.0, 2020),
             (0.0, 0.0, 400.0, 2025), (19.8, -155.5, 4.2, 1905)]
    out = []
    for i, (lat, lon, h, y) in enumerate(cases, start):
        be, bn, bu = ppigrf.igrf(lon, lat, h, datetime(y, 1, 1))
        x, yy, z = float(np.ravel(bn)[0]), float(np.ravel(be)[0]), -float(np.ravel(bu)[0])
        hh = (x * x + yy * yy) ** 0.5
        import math
        inp = {"lat": lat, "lon": lon, "height": f"{h} km", "date": f"{y}-01-01", "model": "igrf14"}
        exp = {"result.north": x, "result.east": yy, "result.down": z,
               "result.declination.value": math.degrees(math.atan2(yy, x)),
               "result.inclination.value": math.degrees(math.atan2(z, hh))}
        tol = {k: {"abs": 1e-3} for k in exp}
        tol["result.declination.value"] = tol["result.inclination.value"] = {"abs": 1e-6}
        out.append(vec(i, inp, exp, tol, IGRF_SRC, IGRF_VER))
    return out


def variation_vectors():
    src = "Arithmetic of FAA-H-8083-25C (magnetic = true - east variation; east is least, west is best)"
    ver = "FAA-H-8083-25C (2023)"
    cases = [(90, "12°W", "true-to-magnetic", 102), (90, "12W", "true-to-magnetic", 102), (355, "10 E", "true-to-magnetic", 345),
             (5, "8°W", "true-to-magnetic", 13), (102, "12°W", "magnetic-to-true", 90), (3, "-7.5", "true-to-magnetic", 10.5)]
    out = []
    for i, (b, v, d, want) in enumerate(cases, 1):
        out.append(vec(i, {"bearing": b, "variation": v, "direction": d}, {"result.result.value": want},
                       {"result.result.value": {"abs": 1e-9}}, src, ver))
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else ROOT / "core/vectors")
    decl = wmm_vectors()
    decl += igrf_vectors(len(decl) + 1)
    files = {"geodesy.magnetic.declination": decl, "geodesy.magnetic.true-to-magnetic": variation_vectors()}
    for tool, vs in files.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
