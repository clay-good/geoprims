#!/usr/bin/env python3
"""Differential data and golden vectors for Maidenhead, GARS, GEOREF, and USNG.

GARS and GEOREF come from GeographicLib's C++ GARS and Georef classes through
tools/vectors/gridref_ref.cpp (compiled here against the installed library).
Maidenhead comes from an independent implementation of the IARU definition
below. USNG comes from GeographicLib's GeoConvert (MGRS, spaced). Longitude
exactly 180 is left out of the GEOREF data: GeographicLib indexes past its
last tile there, and geoprims wraps it to 180° W.

Writes core/crates/gp-geo/tests/data/gridref_diff.txt and the
geodesy.grid-ref.{maidenhead,gars,georef,usng}-{forward,inverse} vectors.
"""
import json
import random
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "core/crates/gp-geo/tests/data/gridref_diff.txt"
OUT = ROOT / "core/vectors"
BIN = Path("/tmp") / "gp_gridref_ref"
GLIB = ("GeographicLib GARS and Georef classes (tools/vectors/gridref_ref.cpp)", "GeographicLib 2.7")
IARU = ("IARU Maidenhead locator definition, independent Python implementation (tools/vectors/gen_gridref.py)", "IARU Region 1 VHF Managers Handbook, version 10.03")
GEOCONVERT = ("GeographicLib GeoConvert -m (MGRS, written with spaces as USNG)", "GeographicLib 2.7")
# Published examples.
WIKI_MH = ("Wikipedia, Maidenhead Locator System (W1AW, Newington, Connecticut, is in FN31pr)", "retrieved 2026-09-19")
WIKI_GEOREF = ("Wikipedia, World Geographic Reference System (NAS Patuxent River, 38.286108, -76.4291704, is at GJPJ3417)", "retrieved 2026-09-19")
NGA_GARS = ("NGA, Global Area Reference System description (180°E to 179°30'W is band 001, 179°30'W to 179°W band 002; 90°S to 89°30'S is band AA, 89°30'S to 89°S band AB)",
            "earth-info.nga.mil, October 6, 2006")
FGDC_USNG = ("FGDC-STD-011-2001, United States National Grid, section 5.2.2 and table 1 (the Washington Monument)", "December 2001")


def build():
    subprocess.run(["c++", "-std=c++17", "-O2", "-I/opt/homebrew/include", "-L/opt/homebrew/lib", "-lGeographicLib",
                    str(ROOT / "tools/vectors/gridref_ref.cpp"), "-o", str(BIN)], check=True)


def ref(lines):
    out = subprocess.run([str(BIN)], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True).stdout
    return out.splitlines()


def maidenhead(lat, lon, chars):
    # A value within a millionth of a finest cell of a grid line is on the line.
    def cell(v):
        return round(v) if abs(v - round(v)) < 1e-6 else int(v // 1)
    x = min(max(cell((lon + 180) * 2880), 0), 360 * 2880 - 1)
    y = min(max(cell((lat + 90) * 5760), 0), 180 * 5760 - 1)
    divs = [18, 10, 24, 10, 24]
    out = ""
    for k in range(chars // 2):
        below = 1
        for d in divs[k + 1:]:
            below *= d
        ix, iy = (x // below) % divs[k], (y // below) % divs[k]
        base = "A" if k == 0 else ("a" if k in (2, 4) else "0")
        out += chr(ord(base) + ix) + chr(ord(base) + iy)
    return out


def vec(i, inp, exp, src, tol=None, ok=True):
    e = dict(exp)
    e["ok"] = ok
    v = {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src[0], "sourceVersion": src[1]}
    if tol:
        v["tolerance"] = tol
    return v


def write(name, vs):
    (OUT / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))
    print(name, len(vs))


def main():
    build()
    rnd = random.Random(20260919)
    pts = [(rnd.uniform(-90, 90), rnd.uniform(-180, 180)) for _ in range(400)]
    pts += [(90, 0), (-90, 0), (0, -180), (89.999999, 179.999999), (-89.999999, -179.999999), (0, 0)]
    # GARS forward at every precision, GEOREF forward at every precision.
    glines = [f"g {la!r} {lo!r} {p}" for la, lo in pts for p in (0, 1, 2)]
    rlines = [f"r {la!r} {lo!r} {p}" for la, lo in pts for p in range(-1, 12)]
    gcodes = ref(glines)
    rcodes = ref(rlines)
    # Decodes of what was encoded.
    gdec = ref([f"G {c}" for c in gcodes])
    rdec = ref([f"R {c}" for c in rcodes])
    rows = []
    for line, code, dec in zip(glines, gcodes, gdec):
        rows.append(f"{line} {code} {dec}")
    for line, code, dec in zip(rlines, rcodes, rdec):
        rows.append(f"{line} {code} {dec}")
    DATA.parent.mkdir(parents=True, exist_ok=True)
    DATA.write_text("\n".join(rows) + "\n")
    print(len(rows), "differential rows")

    # Golden vectors.
    pitt = (40.446111, -79.982222)
    samples = [pitt, (57.64911, 10.40744), (90, 180), (-90, -180), (0, 0), (-33.8688, 151.2093), (64.1466, -21.9426)]
    samples += [(round(rnd.uniform(-89.9, 89.9), 6), round(rnd.uniform(-179.9, 179.9), 6)) for _ in range(14)]

    vs = []
    for i, (la, lo) in enumerate(samples, 1):
        chars = [6, 6, 4, 4, 2, 8, 10][i % 7]
        exp = {"result.locator": maidenhead(la, lo, chars)}
        vs.append(vec(i, {"lat": la, "lon": lo, "precision": str(chars)}, exp, IARU))
    vs.append(vec(len(vs) + 1, {"lat": 95, "lon": 0}, {"error.code": "INVALID_INPUT"}, ("add-geodesy-suite input rules", "2026-09"), ok=False))
    vs.append(vec(len(vs) + 1, {"lat": 41.71463, "lon": -72.72713, "precision": "6"}, {"result.locator": "FN31pr"}, WIKI_MH))
    write("geodesy.grid-ref.maidenhead-forward", vs)

    vs = []
    for i, (la, lo) in enumerate(samples, 1):
        chars = [6, 4, 2, 8, 10][i % 5]
        loc = maidenhead(la, lo, chars)
        if i % 3 == 0:
            loc = loc.swapcase()
        up = maidenhead(la, lo, chars)
        # Decode independently: the cell from the locator's characters.
        divs = [18, 10, 24, 10, 24]
        west, south, w, h = -180.0, -90.0, 360.0, 180.0
        for k in range(chars // 2):
            a, b = up[2 * k], up[2 * k + 1]
            base = "A" if k == 0 else ("a" if k in (2, 4) else "0")
            ix, iy = ord(a) - ord(base), ord(b) - ord(base)
            w /= divs[k]; h /= divs[k]
            west += ix * w; south += iy * h
        exp = {"result.lat.value": south + h / 2, "result.lon.value": west + w / 2, "result.south.value": south, "result.west.value": west,
               "result.north.value": south + h, "result.east.value": west + w}
        tol = {k: {"abs": 1e-12} for k in exp}
        vs.append(vec(i, {"locator": loc}, exp, IARU, tol))
    for j, bad in enumerate(["FN0", "SN00", "FN00zz"], len(vs) + 1):
        vs.append(vec(j, {"locator": bad}, {"error.code": "INVALID_INPUT"}, ("add-geodesy-suite input rules", "2026-09"), ok=False))
    # The subsquare holds W1AW: its center is within half a subsquare (1/48° by 1/24°) of the station.
    vs.append(vec(len(vs) + 1, {"locator": "FN31pr"}, {"result.lat.value": 41.71463, "result.lon.value": -72.72713}, WIKI_MH,
                  {"result.lat.value": {"abs": 1 / 48}, "result.lon.value": {"abs": 1 / 24}}))
    write("geodesy.grid-ref.maidenhead-inverse", vs)

    names = {0: "30min", 1: "15min", 2: "5min"}
    lines = [f"g {la!r} {lo!r} {i % 3}" for i, (la, lo) in enumerate(samples)]
    codes = ref(lines)
    decs = ref([f"G {c}" for c in codes])
    vs, vi = [], []
    for i, ((la, lo), code, dec) in enumerate(zip(samples, codes, decs), 1):
        s_, w_, cl, cw, _ = map(float, dec.split())
        vs.append(vec(i, {"lat": la, "lon": lo, "precision": names[(i - 1) % 3]}, {"result.gars": code}, GLIB))
        exp = {"result.lat.value": cl, "result.lon.value": cw, "result.south.value": s_, "result.west.value": w_}
        vi.append(vec(i, {"gars": code.lower() if i % 4 == 0 else code}, exp, GLIB, {k: {"abs": 1e-12} for k in exp}))
    vi.append(vec(len(vi) + 1, {"gars": "721AA"}, {"error.code": "INVALID_INPUT"}, ("add-geodesy-suite input rules", "2026-09"), ok=False))
    vi.append(vec(len(vi) + 1, {"gars": "001RA"}, {"error.code": "INVALID_INPUT"}, ("add-geodesy-suite input rules", "2026-09"), ok=False))
    for code, la, lo in [("001AA", -89.75, -179.75), ("002AB", -89.25, -179.25)]:
        vs.append(vec(len(vs) + 1, {"lat": la, "lon": lo, "precision": "30min"}, {"result.gars": code}, NGA_GARS))
        vi.append(vec(len(vi) + 1, {"gars": code}, {"result.south.value": la - 0.25, "result.west.value": lo - 0.25, "result.lat.value": la, "result.lon.value": lo},
                      NGA_GARS, {k: {"abs": 1e-12} for k in ["result.south.value", "result.west.value", "result.lat.value", "result.lon.value"]}))
    write("geodesy.grid-ref.gars-forward", vs)
    write("geodesy.grid-ref.gars-inverse", vi)

    precs = ["15deg", "1deg", "1min", "0.1min", "0.01min", "0.001min"]
    pnum = {"15deg": -1, "1deg": 0, "1min": 2, "0.1min": 3, "0.01min": 4, "0.001min": 5}
    samp = [(la, lo) for la, lo in samples if lo != 180]
    lines = [f"r {la!r} {lo!r} {pnum[precs[i % 6]]}" for i, (la, lo) in enumerate(samp)]
    codes = ref(lines)
    decs = ref([f"R {c}" for c in codes])
    vs, vi = [], []
    for i, ((la, lo), code, dec) in enumerate(zip(samp, codes, decs), 1):
        s_, w_, cl, cw, _ = map(float, dec.split())
        vs.append(vec(i, {"lat": la, "lon": lo, "precision": precs[(i - 1) % 6]}, {"result.georef": code}, GLIB))
        exp = {"result.lat.value": cl, "result.lon.value": cw, "result.south.value": s_, "result.west.value": w_}
        vi.append(vec(i, {"georef": code}, exp, GLIB, {k: {"abs": 1e-12} for k in exp}))
    vi.append(vec(len(vi) + 1, {"georef": "NKLN6099"}, {"error.code": "INVALID_INPUT"}, ("add-geodesy-suite input rules", "2026-09"), ok=False))
    vs.append(vec(len(vs) + 1, {"lat": 38.286108, "lon": -76.4291704, "precision": "1min"}, {"result.georef": "GJPJ3417"}, WIKI_GEOREF))
    # The 1-minute cell holds the station: its center is within half a minute of it.
    vi.append(vec(len(vi) + 1, {"georef": "GJPJ3417"}, {"result.lat.value": 38.286108, "result.lon.value": -76.4291704}, WIKI_GEOREF,
                  {"result.lat.value": {"abs": 1 / 120}, "result.lon.value": {"abs": 1 / 120}}))
    write("geodesy.grid-ref.georef-forward", vs)
    write("geodesy.grid-ref.georef-inverse", vi)

    # USNG: GeoConvert's MGRS, spaced. Precision 5 (1 m) unless noted.
    usamp = [(la, lo) for la, lo in samples if abs(la) < 84]
    out = subprocess.run(["GeoConvert", "-m", "-p", "0"], input="".join(f"{la!r} {lo!r}\n" for la, lo in usamp), capture_output=True, text=True, check=True).stdout.split()
    def spaced(m):
        # 17TNE8630977770 -> 17T NE 86309 77770
        k = 0
        while m[k].isdigit():
            k += 1
        gzd, sq, digits = m[:k + 1], m[k + 1:k + 3], m[k + 3:]
        h = len(digits) // 2
        return " ".join(x for x in (gzd, sq, digits[:h], digits[h:]) if x)
    vs = [vec(i, {"lat": la, "lon": lo}, {"result.usng": spaced(m)}, GEOCONVERT) for i, ((la, lo), m) in enumerate(zip(usamp, out), 1)]
    # NAD 83 UTM 18 E 323,483.168 m, N 4,306,479.498 m; its latitude and longitude from PROJ 9.3.0.
    monument = (38.889467309501576, -77.0352402156242)
    for p, ref_ in [("1m", "18S UJ 23483 06479"), ("10m", "18S UJ 2348 0647"), ("100m", "18S UJ 234 064"), ("1km", "18S UJ 23 06")]:
        vs.append(vec(len(vs) + 1, {"lat": monument[0], "lon": monument[1], "precision": p}, {"result.usng": ref_}, FGDC_USNG))
    write("geodesy.grid-ref.usng-forward", vs)
    vi = []
    for i, ((la, lo), m) in enumerate(zip(usamp, out), 1):
        dec = subprocess.run(["GeoConvert", "-p", "9"], input=m + "\n", capture_output=True, text=True, check=True).stdout.split()
        exp = {"result.lat.value": float(dec[0]), "result.lon.value": float(dec[1])}
        vi.append(vec(i, {"usng": spaced(m)}, exp, GEOCONVERT, {k: {"abs": 1e-9} for k in exp}))
    # A truncated local reference in a known grid zone (the Pittsburgh zone).
    loc = subprocess.run(["GeoConvert", "-p", "9"], input="17TNE863777\n", capture_output=True, text=True, check=True).stdout.split()
    vi.append(vec(len(vi) + 1, {"usng": "NE 863 777", "zone": "17T"}, {"result.lat.value": float(loc[0]), "result.lon.value": float(loc[1]), "result.square_size.value": 100},
                  GEOCONVERT, {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}, "result.square_size.value": {"abs": 0}}))
    vi.append(vec(len(vi) + 1, {"usng": "NE 863 777"}, {"error.code": "INVALID_INPUT"}, ("add-geodesy-suite input rules", "2026-09"), ok=False))
    vi.append(vec(len(vi) + 1, {"usng": "18S UJ 23483168 06479498"}, {"result.corner_lat.value": monument[0], "result.corner_lon.value": monument[1],
                  "result.square_size.value": 0.001}, FGDC_USNG, {"result.corner_lat.value": {"abs": 1e-10}, "result.corner_lon.value": {"abs": 1e-10},
                  "result.square_size.value": {"abs": 0}}))
    write("geodesy.grid-ref.usng-inverse", vi)


if __name__ == "__main__":
    main()
