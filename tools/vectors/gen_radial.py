#!/usr/bin/env python3
"""Golden vectors for the radial fix: true course = radial + station
variation, the fix by GeographicLib's GeodSolve (C++), and the WMM2025
declination at the station on 2026-09-22 as geodesy.magnetic.declination
gives it (that tool is checked against NOAA's WMM2025 test values).
Requires GeodSolve."""
import json
import subprocess
import sys
from pathlib import Path

VER = subprocess.run(["GeodSolve", "--version"], capture_output=True, text=True).stdout.strip() or "GeographicLib"
SRC = "GeodSolve fixes and WMM2025 declinations (tools/vectors/gen_radial.py)"
SPEC = "add-practitioner-essentials variation scenario"
NM = 1852.0
# WMM2025 declination on 2026-09-22 at sea level, from geodesy.magnetic.declination.
WMM = {(39.8, -104.7): 7.353641447334012, (47.45, -122.3): 14.837867042643614, (25.8, -80.3): -7.323346975644197, (64.8, -147.9): 14.73049966298234}


def direct(lat, lon, az, s):
    line = " ".join(f"{float(x):.15f}" for x in (lat, lon, az, s))
    return [float(x) for x in subprocess.run(["GeodSolve", "-p", "12"], input=line + "\n", capture_output=True, text=True, check=True).stdout.split()][:2]


def vec(i, inp, exp, src=SRC):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": 1e-9} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": VER, "tolerance": t}


def cases():
    out = []
    for (lat, lon), radial, nm, var, warn in [((39.8, -104.7), 98, 12.5, 11, True), ((47.45, -122.3), 270, 30, 16, True),
                                              ((25.8, -80.3), 45, 8, -7, False), ((64.8, -147.9), 180, 55, 15, False)]:
        tc = (radial + var) % 360
        flat, flon = direct(lat, lon, tc, nm * NM)
        d = WMM[(lat, lon)]
        exp = {"result.fix_lat.value": flat, "result.fix_lon.value": flon, "result.true_course.value": tc,
               "result.wmm_declination.value": d, "result.variation_difference.value": var - d}
        if warn:
            exp["meta.warnings.1.code"] = "STATION_VARIATION_DIFFERS"
        out.append(vec(len(out) + 1, {"lat": lat, "lon": lon, "radial": f"{radial} deg", "distance": f"{nm} NM", "variation": f"{var} deg", "date": "2026-09-22"}, exp))
    out[0]["source"] = SPEC + " (the station's variation is used, and a difference over 1° from WMM is warned)"
    flat, flon = direct(39.8, -104.7, 109, 12.5 * NM)
    out.append(vec(len(out) + 1, {"lat": 39.8, "lon": -104.7, "radial": "98 deg", "distance": "12.5 NM", "variation": "11 deg"},
                   {"result.fix_lat.value": flat, "result.fix_lon.value": flon, "result.true_course.value": 109.0}))
    out.append(vec(len(out) + 1, {"lat": 39.8, "lon": -104.7, "radial": "98 deg", "distance": "12.5 NM", "variation": "11 deg", "date": "2035-01-01"}, {"ok": False, "error.code": "OUT_OF_DOMAIN"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "aviation.ifr.radial-fix.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in cases()))


if __name__ == "__main__":
    main()
