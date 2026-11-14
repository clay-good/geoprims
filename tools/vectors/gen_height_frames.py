#!/usr/bin/env python3
"""Golden vectors for the frame of an ellipsoidal height in
geodesy.height.ellipsoidal-to-orthometric, orthometric-to-ellipsoidal, and
convert (add-geodesy-suite 6.3, geodesy/heights-and-geoid).

EGM96 is referred to WGS 84, and a US survey height is on NAD 83 (2011). With
an epoch the tools move the height to WGS 84 before using the geoid; without
one they warn FRAME_MISMATCH. Two independent references:

- The frame step from NGS HTDP itself, compiled from its Fortran source by
  gen_htdp.py's own build() and htdp() (imported, not written again), menu
  option 4 between NAD83(2011) and WGS84(G2296) with equal input and output
  dates, so only the frame transformation applies.
- The geoid height N from GeographicLib's GeoidEval on the repository's own
  egm96-15 grid, cubic, as gen_geoid_diff.py runs it.

    gen_height_frames.py      # print each file's rows, numbered from 1

The published files are frozen: append these rows by hand, renumbered to
continue each file.
"""
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen_htdp  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
GRID = ROOT / "assets/data/egm96-15/2009-08-29"
SRC = "NGS HTDP, compiled from htdp.f and initbd.f, menu option 4, for the frame step; GeographicLib GeoidEval with the egm96-15 grid for N (tools/vectors/gen_height_frames.py)"
VER = "HTDP 3.6.0 (2025-04-07); egm96-15 (2009-08-29)"
# HTDP prints heights to the millimetre; the tolerance is that rounding plus a
# millimetre for HTDP's own arithmetic, as for geodesy.datum.nad83.
HT = {"abs": 0.0015}
N_TOL = {"abs": 5e-05}

# Places across the conterminous United States, Alaska, and Hawaii, where a
# NAD 83 (2011) height and a WGS 84 one differ by about a metre or more.
PLACES = [
    ("An OPUS mark in Kansas", 38.5, -98.0, 500.0, 2026.7),
    ("Denver", 39.7392, -104.9903, 1609.0, 2026.0),
    ("Pittsburgh", 40.446111, -79.982222, 300.0, 2024.5),
    ("Seattle", 47.6062, -122.3321, 50.0, 2026.87),
    ("Miami", 25.7617, -80.1918, 2.0, 2020.0),
]


def geoid(points):
    inp = "".join(f"{la!r} {lo!r}\n" for la, lo in points)
    out = subprocess.run(["GeoidEval", "-d", str(GRID), "-n", "egm96-15"], input=inp,
                         capture_output=True, text=True, check=True).stdout.split()
    return [float(x) for x in out]


def rows():
    """(tool, row) pairs, in the order they are appended."""
    out = []
    with tempfile.TemporaryDirectory() as d:
        work = Path(d)
        exe = gen_htdp.build(work)
        ns = geoid([(la, lo) for _, la, lo, _, _ in PLACES])
        for (name, lat, lon, h, t), n in zip(PLACES, ns):
            # h -> H: NAD 83 (2011) height to WGS 84, then less N.
            (_, _, wgs), = gen_htdp.htdp(exe, work, "NAD83(2011)", "WGS84(G2296)", t, [(lat, lon, h)])
            orth = wgs - n
            exp = {"result.orthometric.value": orth, "result.geoid_height.value": n,
                   "result.ellipsoidal.value": h, "result.frame_step.value": wgs - h}
            tol = {"result.orthometric.value": HT, "result.geoid_height.value": N_TOL,
                   "result.ellipsoidal.value": {"abs": 1e-9}, "result.frame_step.value": HT}
            inp = {"lat": lat, "lon": lon, "height": f"{h} m", "frame": "NAD83(2011)", "epoch": str(t)}
            out.append(("geodesy.height.ellipsoidal-to-orthometric",
                        {"input": inp, "expect": {"ok": True, **exp}, "source": f"{SRC}: {name}",
                         "sourceVersion": VER, "tolerance": tol}))
            out.append(("geodesy.height.convert",
                        {"input": {**inp, "from": "ellipsoidal"},
                         "expect": {"ok": True, **exp, "result.converted.value": orth},
                         "source": f"{SRC}: {name}", "sourceVersion": VER,
                         "tolerance": {**tol, "result.converted.value": HT}}))
            # H -> h: H + N is a WGS 84 height, moved to NAD 83 (2011). The
            # orthometric height is the one the forward row found, so the
            # pair is a round trip through the oracle.
            H = round(orth, 3)
            (_, _, nad), = gen_htdp.htdp(exe, work, "WGS84(G2296)", "NAD83(2011)", t, [(lat, lon, H + n)])
            out.append(("geodesy.height.orthometric-to-ellipsoidal",
                        {"input": {"lat": lat, "lon": lon, "height": f"{H} m", "frame": "NAD83(2011)", "epoch": str(t)},
                         "expect": {"ok": True, "result.ellipsoidal.value": nad, "result.geoid_height.value": n,
                                    "result.orthometric.value": H, "result.frame_step.value": H + n - nad},
                         "source": f"{SRC}: {name}, back", "sourceVersion": VER,
                         "tolerance": {"result.ellipsoidal.value": HT, "result.geoid_height.value": N_TOL,
                                       "result.orthometric.value": {"abs": 1e-9},
                                       "result.frame_step.value": HT}}))
        # Without an epoch the step is not applied: the answer is the WGS 84
        # one, and FRAME_MISMATCH says so (geodesy/heights-and-geoid, scenario
        # "Frame mismatch").
        name, lat, lon, h, _ = PLACES[0]
        n = ns[0]
        out.append(("geodesy.height.ellipsoidal-to-orthometric",
                    {"input": {"lat": lat, "lon": lon, "height": f"{h} m", "frame": "NAD83(2011)"},
                     "expect": {"ok": True, "result.orthometric.value": h - n,
                                "meta.warnings.*.code": "FRAME_MISMATCH"},
                     "source": f"GeographicLib GeoidEval with the egm96-15 grid (tools/vectors/gen_height_frames.py): {name}, with no epoch, so no step and FRAME_MISMATCH",
                     "sourceVersion": "egm96-15 (2009-08-29)",
                     "tolerance": {"result.orthometric.value": N_TOL}}))
        out.append(("geodesy.height.ellipsoidal-to-orthometric",
                    {"input": {"lat": lat, "lon": lon, "height": f"{h} m", "frame": "NAD83(2011)", "epoch": "1900"},
                     "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
                     "source": "add-geodesy-suite input rules: the epoch, as for geodesy.datum.nad83, is 1980 to 2100",
                     "sourceVersion": "HTDP 3.6.0 (2025-04-07)", "tolerance": {}}))
    return out


def main():
    counts = {}
    for tool, r in rows():
        counts[tool] = counts.get(tool, 0) + 1
        sys.stdout.write(f"{tool}\t" + json.dumps({"id": f"v{counts[tool]:03d}", **r},
                                                  ensure_ascii=False, separators=(",", ":")) + "\n")


if __name__ == "__main__":
    main()
