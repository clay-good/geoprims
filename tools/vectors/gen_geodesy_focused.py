#!/usr/bin/env python3
"""Golden vectors for the focused geodesy operations that share an engine with
a general tool: the three coordinate formatters, the two single-notation
parsers, magnetic-to-true, meridian arc, and degree lengths.

Formatting and parsing use exact decimal arithmetic; meridian arcs come from
GeographicLib's GeodSolve CLI along a meridian, which is independent of the
core's rectifying-latitude series. Needs GeodSolve on the PATH."""
import json
import math
import random
import subprocess
import sys
from decimal import ROUND_HALF_UP, Decimal
from fractions import Fraction
from pathlib import Path

EXACT = ("Exact decimal arithmetic (tools/vectors/gen_geodesy_focused.py)", "definition")
SPEC = ("add-geodesy-suite scenarios", "2026")
GEOD_VER = subprocess.run(["GeodSolve", "--version"], capture_output=True, text=True).stdout.strip() or "GeographicLib"
GEOD = ("GeographicLib GeodSolve along a meridian, and the closed-form prime-vertical radius (tools/vectors/gen_geodesy_focused.py)", GEOD_VER)
ELLIPSOIDS = {None: (6378137.0, 298.257223563), "grs80": (6378137.0, 298.257222101), "clarke1866": (6378206.4, 294.978698213898),
              "intl1924": (6378388.0, 297.0), "bessel1841": (6377397.155, 299.1528128)}


def vec(i, inp, exp, src, tol=None):
    e = dict(exp)
    e.setdefault("ok", True)
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src[0], "sourceVersion": src[1],
            "tolerance": {k: tol for k, v in e.items() if isinstance(v, float)} if tol else {}}


def refusal(i, inp, code="INVALID_INPUT", field=None):
    e = {"ok": False, "error.code": code}
    if field:
        e["error.field"] = field
    return vec(i, inp, e, SPEC)


# ---------------------------------------------------------------- formatting

def parts(value, style, decimals):
    """(degrees, minutes, seconds) of |value| rounded half-up at `decimals` on the last part, with carries."""
    q = Decimal(1).scaleb(-decimals)
    x = abs(Decimal(repr(value)))
    if style == "dd":
        return (x.quantize(q, ROUND_HALF_UP),)
    if style == "ddm":
        minutes = (x * 60).quantize(q, ROUND_HALF_UP)
        return (int(minutes // 60), minutes - 60 * (minutes // 60))
    seconds = (x * 3600).quantize(q, ROUND_HALF_UP)
    return (int(seconds // 3600), int(seconds // 60 % 60), seconds - 60 * (seconds // 60))


def tie(value, style, decimals):
    """True when the digit after the last kept one is an exact 5: the one case two correct formatters may round differently."""
    x = abs(Decimal(repr(value))) * {"dd": 1, "ddm": 60, "dms": 3600}[style]
    return (x.scaleb(decimals) % 1) == Decimal("0.5")


def write(value, positive, negative, style, decimals, letters=True):
    p = parts(value, style, decimals)
    width = 2 + (decimals + 1 if decimals else 0)
    if style == "dd":
        body = f"{p[0]:.{decimals}f}°"
    elif style == "ddm":
        body = f"{p[0]}°{p[1]:0{width}.{decimals}f}'"
    else:
        body = f"{p[0]}°{p[1]:02d}'{p[2]:0{width}.{decimals}f}\""
    if letters:
        return body + (negative if value < 0 else positive)
    return ("-" if value < 0 else "") + body


def format_vectors(style):
    rng = random.Random({"dd": 66101, "ddm": 66102, "dms": 66103}[style])
    first = {"dd": (-33.856784, 151.215297, 5), "dms": (48.858370, 2.294481, 1), "ddm": (-33.906389, 18.420833, 3)}[style]
    cases = [first]
    while len(cases) < 20:
        lat, lon = round(rng.uniform(-89, 89), 6), round(rng.uniform(-179, 179), 6)
        decimals = rng.randrange(0, {"dd": 8, "ddm": 6, "dms": 4}[style])
        if abs(lat) < 0.01 or abs(lon) < 0.01 or tie(lat, style, decimals) or tie(lon, style, decimals):
            continue
        cases.append((lat, lon, decimals))
    out = []
    for i, (lat, lon, decimals) in enumerate(cases, 1):
        signed = style == "dd" and i % 4 == 0
        text = f"{write(lat, 'N', 'S', style, decimals, not signed)} {write(lon, 'E', 'W', style, decimals, not signed)}"
        inp = {"lat": lat, "lon": lon, "decimals": decimals}
        if signed:
            inp["signs"] = "signed"
        out.append(vec(i, inp, {"result.formatted": text}, EXACT))
    out.append(refusal(len(out) + 1, {"lat": 91, "lon": 0}))
    out.append(refusal(len(out) + 1, {"lat": 40, "lon": -75, "decimals": 2.5}, field="/decimals"))
    return out


# ---------------------------------------------------------------- parsing

def parse_vectors(notation):
    rng = random.Random({"dms": 66104, "ddm": 66105}[notation])
    out = []

    def one(deg, minutes, seconds, hemi):
        if notation == "dms":
            value = Fraction(deg) + Fraction(minutes, 60) + Fraction(str(seconds)) / 3600
            text = f"{deg}°{minutes:02d}'{seconds}\"{hemi}"
        else:
            value = Fraction(deg) + Fraction(str(minutes)) / 60
            text = f"{deg}°{minutes}'{hemi}"
        return text, float(-value if hemi in "SW" else value)

    first = {"dms": ((33, 56, "46", "S"), (151, 10, "38", "E")), "ddm": ((38, "41.250", None, "N"), (9, "25.500", None, "W"))}[notation]
    cases = [first]
    while len(cases) < 20:
        if notation == "dms":
            mk = lambda limit, hemis: (rng.randrange(0, limit), rng.randrange(0, 60), f"{rng.uniform(0, 59.9):.{rng.randrange(0, 4)}f}", rng.choice(hemis))
        else:
            mk = lambda limit, hemis: (rng.randrange(0, limit), f"{rng.uniform(0, 59.9):.{rng.randrange(1, 5)}f}", None, rng.choice(hemis))
        cases.append((mk(89, "NS"), mk(179, "EW")))
    for i, (la, lo) in enumerate(cases, 1):
        lat_text, lat = one(*la)
        lon_text, lon = one(*lo)
        # Vary how the pair is written: a space, a comma, or the hemisphere first.
        text = [f"{lat_text} {lon_text}", f"{lat_text}, {lon_text}", f"{lat_text[-1]} {lat_text[:-1]} {lon_text[-1]} {lon_text[:-1]}"][i % 3]
        out.append(vec(i, {"text": text}, {"result.lat.value": lat, "result.lon.value": lon, "result.notation": notation.upper()}, EXACT, {"abs": 1e-12}))
    other = "40°26.767'N 79°58.933'W" if notation == "dms" else "40°26'46\"N 79°58'56\"W"
    for text in [other, "40.446111, -79.982222", "17TNE8461877884", "not a coordinate"]:
        out.append(refusal(len(out) + 1, {"text": text}, field="/text"))
    return out


# ---------------------------------------------------------------- magnetic to true

def magnetic_vectors():
    cases = [(215, "9°E", 9), (90, "12°W", -12), (0, "3.5 E", 3.5), (359, "2°E", 2), (1, "5°W", -5), (180, "0", 0), (45.5, "-12", -12),
             (270, "14.2 W", -14.2), (33, "21°30'E", 21.5), (310, "7°15'W", -7.25), (5, "10°W", -10), (355, "10°E", 10), (123.4, "0.5 E", 0.5),
             (200, "18 W", -18), (60, "25°E", 25), (300, "30°W", -30), (179.9, "0.2 E", 0.2), (0.1, "0.2 W", -0.2), (88, "4°45'E", 4.75), (240, "11 E", 11)]
    out = []
    for i, (bearing, text, variation) in enumerate(cases, 1):
        src = ("Arithmetic of FAA-H-8083-25C (true = magnetic + east variation; east is least, west is best)", "FAA-H-8083-25C (2023)")
        out.append(vec(i, {"bearing": f"{bearing} deg", "variation": text},
                       {"result.result.value": float((bearing + variation) % 360), "result.variation_used.value": float(variation), "result.variation_source": "chart"},
                       src, {"abs": 1e-9}))
    out.append(refusal(len(out) + 1, {"bearing": "90 deg"}, field="/variation"))
    out.append(refusal(len(out) + 1, {"bearing": "90 deg", "variation": "north"}, field="/variation"))
    return out


# ---------------------------------------------------------------- ellipsoid

def meridian_arc(lats, a, rf):
    """Signed meridian arc from the equator for each latitude, by GeodSolve along the prime meridian."""
    text = "".join(f"0 0 {lat:.12f} 0\n" for lat in lats)
    out = subprocess.run(["GeodSolve", "-i", "-p", "9", "-e", f"{a:.6f}", f"{1 / rf:.15f}"], input=text, capture_output=True, text=True, check=True).stdout
    return [math.copysign(float(line.split()[2]), lat) if lat else 0.0 for lat, line in zip(lats, out.strip().splitlines())]


def degree_lat(lat, a, rf):
    lo, hi = max(lat - 0.5, -90.0), min(lat + 0.5, 90.0)
    m_lo, m_hi = meridian_arc([lo, hi], a, rf)
    return (m_hi - m_lo) / (hi - lo)


def ellipsoid_inp(lat, name, extra=None):
    inp = {"lat": lat, **(extra or {})}
    if name:
        inp["ellipsoid"] = name
    return inp


def arc_vectors():
    cases = [(40, 41, None), (45, None, None), (0, 90, None), (-33.5, None, None), (60, 59, None), (10, -10, None), (89.5, None, None),
             (51.4778, 48.8566, None), (-45, -46, None), (23.4367, None, None), (35, 36, "grs80"), (40, None, "clarke1866"), (52, 53, "intl1924"),
             (48, None, "bessel1841"), (-75, -60, None), (1, None, None), (66.5633, 90, None), (-0.5, 0.5, None), (30, 31, "clarke1866"), (15.25, 15.75, None)]
    out = []
    for i, (lat, lat2, name) in enumerate(cases, 1):
        a, rf = ELLIPSOIDS[name]
        exp = {"result.meridian_arc.value": meridian_arc([lat], a, rf)[0], "result.degree_lat.value": degree_lat(lat, a, rf)}
        extra = {}
        if lat2 is not None:
            extra["lat2"] = lat2
            exp["result.arc_between.value"] = meridian_arc([lat2], a, rf)[0] - exp["result.meridian_arc.value"]
        out.append(vec(i, ellipsoid_inp(lat, name, extra), exp, GEOD, {"abs": 2e-6}))
    out.append(refusal(len(out) + 1, {"lat": 91}))
    out.append(refusal(len(out) + 1, {"lat": 40, "lat2": -95}))
    return out


def degree_vectors():
    cases = [(60, None), (0, None), (45, None), (-33.8688, None), (89, None), (23.5, None), (51.4778, None), (-45, None), (10, None), (75, None),
             (40.7128, "grs80"), (35, "clarke1866"), (48.8566, "intl1924"), (52.52, "bessel1841"), (-1.2921, None), (64.1466, None), (30, None),
             (-54.8, None), (19.4326, "clarke1866"), (80, None)]
    out = []
    for i, (lat, name) in enumerate(cases, 1):
        a, rf = ELLIPSOIDS[name]
        f = 1 / rf
        e2 = f * (2 - f)
        phi = math.radians(lat)
        dlat = degree_lat(lat, a, rf)
        dlon = math.radians(1) * a / math.sqrt(1 - e2 * math.sin(phi) ** 2) * math.cos(phi)
        out.append(vec(i, ellipsoid_inp(lat, name), {"result.degree_lat.value": dlat, "result.degree_lon.value": dlon, "result.arcminute_lat.value": dlat / 60,
                                                    "result.arcminute_lon.value": dlon / 60, "result.arcsecond_lat.value": dlat / 3600,
                                                    "result.arcsecond_lon.value": dlon / 3600}, GEOD, {"abs": 2e-6}))
    out.append(refusal(len(out) + 1, {"lat": -90.5}))
    out.append(refusal(len(out) + 1, {"lat": 45, "a": "6378137 m"}))
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    selected = set(sys.argv[2:])
    files = {"geodesy.parse.format-dd": lambda: format_vectors("dd"), "geodesy.parse.format-dms": lambda: format_vectors("dms"),
             "geodesy.parse.format-ddm": lambda: format_vectors("ddm"), "geodesy.parse.parse-dms": lambda: parse_vectors("dms"),
             "geodesy.parse.parse-ddm": lambda: parse_vectors("ddm"), "geodesy.magnetic.magnetic-to-true": magnetic_vectors,
             "geodesy.ellipsoid.meridian-arc": arc_vectors, "geodesy.ellipsoid.degree-lengths": degree_vectors}
    for tool, make in files.items():
        if selected and tool not in selected:
            continue
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in make()))


if __name__ == "__main__":
    main()
