#!/usr/bin/env python3
"""Golden vectors for geodesy.datum.nad83 from NGS HTDP itself.

Downloads HTDP (https://geodesy.noaa.gov/TOOLS/Htdp/HTDP-download.zip),
compiles htdp.f and initbd.f with gfortran, and runs menu option 4
(transform positions between reference frames) with equal input and output
dates, so only the frame transformation applies. HTDP prints latitude and
longitude to 1e-10° and heights to the millimeter, so vectors compare within
1e-8° (about 1 mm) and 1.5 mm.

Writes core/vectors/geodesy.datum.nad83.jsonl. Requires gfortran and curl.
"""
import json
import random
import subprocess
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "core/vectors/geodesy.datum.nad83.jsonl"
URL = "https://geodesy.noaa.gov/TOOLS/Htdp/HTDP-download.zip"
SRC = ("NGS HTDP, compiled from htdp.f and initbd.f, menu option 4", "HTDP 3.6.0 (2025-04-07)")
# geoprims frame name -> HTDP menu number.
MENU = {"NAD83(2011)": 1, "NAD83(PA11)": 2, "NAD83(MA11)": 3, "WGS84(G1150)": 8, "WGS84(G1674)": 9, "WGS84(G1762)": 10,
        "WGS84(G2139)": 11, "WGS84(G2296)": 12, "ITRF2000": 22, "ITRF2005": 23, "ITRF2008": 24, "ITRF2014": 25, "ITRF2020": 26}


def build(work):
    z = work / "htdp.zip"
    subprocess.run(["curl", "-sL", "-m", "300", "-o", str(z), URL], check=True)
    zipfile.ZipFile(z).extractall(work)
    subprocess.run(["gfortran", "-O2", "-std=legacy", "-o", str(work / "htdp"), str(work / "htdp.f"), str(work / "initbd.f")], check=True, capture_output=True)
    return work / "htdp"


def htdp(exe, work, a, b, t, pts):
    (work / "pts.txt").write_text("".join(f"{la!r},{-lo!r},{h!r},P{i}\n" for i, (la, lo, h) in enumerate(pts)))
    out = work / "out.txt"
    out.unlink(missing_ok=True)
    script = f"4\nout.txt\n{MENU[a]}\n{MENU[b]}\n2\n{t}\n2\n{t}\n3\npts.txt\n0\n0\n"
    subprocess.run([str(exe)], input=script, text=True, cwd=work, capture_output=True, timeout=60)
    rows = [l.split() for l in out.read_text().splitlines() if l.strip().split()[-1:] and l.strip().split()[-1].startswith("P")]
    # HTDP writes longitude west-positive in [0, 360).
    return [(float(r[0]), (-float(r[1]) + 180) % 360 - 180, float(r[2])) for r in rows]


def vec(i, inp, exp, src, tol=None, ok=True):
    e = dict(exp)
    e["ok"] = ok
    v = {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src[0], "sourceVersion": src[1]}
    if tol:
        v["tolerance"] = tol
    return v


def main():
    rnd = random.Random(20260919)
    with tempfile.TemporaryDirectory() as d:
        work = Path(d)
        exe = build(work)
        groups = [("WGS84(G2296)", "NAD83(2011)", 2026.7, [(38.5, -98.0, 500.0), (40.446111, -79.982222, 300.0), (34.05, -118.25, 90.0), (47.6, -122.33, 50.0)]),
                  ("ITRF2020", "NAD83(2011)", 2010.0, [(39.0, -105.0, 1600.0), (30.27, -97.74, 150.0), (61.2, -149.9, 40.0)]),
                  ("NAD83(2011)", "ITRF2014", 2020.5, [(44.98, -93.27, 260.0), (25.77, -80.19, 0.0)]),
                  ("ITRF2020", "NAD83(PA11)", 2024.0, [(21.3, -157.86, 10.0), (19.72, -155.08, 20.0)]),
                  ("ITRF2020", "NAD83(MA11)", 2024.0, [(13.47, 144.75, 80.0)]),
                  ("NAD83(2011)", "WGS84(G1762)", 2015.0, [(35.0, -90.0, 100.0)]),
                  ("ITRF2008", "ITRF2020", 2012.0, [(-33.87, 151.21, 30.0), (48.85, 2.35, 60.0)]),
                  ("WGS84(G1150)", "ITRF2020", 2005.0, [(51.5, -0.12, 20.0)]),
                  ("WGS84(G1674)", "NAD83(2011)", 2012.0, [(42.36, -71.06, 15.0)])]
        for _ in range(3):
            groups.append(("ITRF2020", "NAD83(2011)", round(rnd.uniform(1995, 2030), 2),
                           [(round(rnd.uniform(25, 49), 6), round(rnd.uniform(-124, -67), 6), round(rnd.uniform(0, 3000), 3)) for _ in range(2)]))
        vs = []
        for a, b, t, pts in groups:
            got = htdp(exe, work, a, b, t, pts)
            assert len(got) == len(pts), (a, b, t, got)
            for (la, lo, h), (la2, lo2, h2) in zip(pts, got):
                exp = {"result.lat.value": la2, "result.lon.value": lo2, "result.height.value": h2}
                tol = {"result.lat.value": {"abs": 1e-8}, "result.lon.value": {"abs": 1e-8}, "result.height.value": {"abs": 0.0015}}
                vs.append(vec(len(vs) + 1, {"from": a, "to": b, "epoch": str(t), "lat": la, "lon": lo, "height": h}, exp, SRC, tol))
    rules = ("add-geodesy-suite input rules", "2026-09")
    vs.append(vec(len(vs) + 1, {"from": "WGS84", "to": "NAD83(2011)", "epoch": "2026.7", "lat": 38.5, "lon": -98.0}, {"meta.warnings.*.code": "REALIZATION_ASSUMED"},
                  ("add-geodesy-suite scenario: unqualified WGS 84", "2026-09")))
    vs.append(vec(len(vs) + 1, {"from": "ITRF2020", "to": "NAD83(2011)", "epoch": "2026.7", "lat": 38.5, "lon": -98.0}, {"meta.context.nad83ReferenceEpoch": 2010},
                  ("add-geodesy-suite scenario: epoch echoed", "2026-09"), {"meta.context.nad83ReferenceEpoch": {"abs": 0}}))
    vs.append(vec(len(vs) + 1, {"from": "ITRF2020", "to": "NAD83(2011)", "epoch": "1900", "lat": 38.5, "lon": -98.0}, {"error.code": "OUT_OF_DOMAIN"}, rules, ok=False))
    OUT.write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))
    print(OUT.name, len(vs))


if __name__ == "__main__":
    main()
