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


# ---------------------------------------------------------------- local frames

TRIG = ("Spherical and Cartesian trigonometry in the local tangent frame (tools/vectors/gen_geodesy_focused.py)", "definition")


def aer_to_enu_vectors():
    rng = random.Random(66201)
    cases = [(135.0, 10.0, 2500.0), (0.0, 0.0, 1000.0), (90.0, 45.0, 100.0), (270.0, -30.0, 5000.0), (45.0, 90.0, 10.0), (180.0, -90.0, 7.5)]
    while len(cases) < 20:
        cases.append((round(rng.uniform(0, 360), 4), round(rng.uniform(-89, 89), 4), round(rng.uniform(1, 400000), 3)))
    out = []
    for i, (az, el, r) in enumerate(cases, 1):
        a, e = math.radians(az), math.radians(el)
        east, north, up = r * math.cos(e) * math.sin(a), r * math.cos(e) * math.cos(a), r * math.sin(e)
        out.append(vec(i, {"azimuth": az, "elevation": el, "range": f"{r} m"},
                       {"result.east.value": east, "result.north.value": north, "result.up.value": up, "result.down.value": -up,
                        "result.horizontal.value": math.hypot(east, north)}, TRIG, {"abs": 1e-7}))
    out.append(refusal(len(out) + 1, {"azimuth": 10, "elevation": 91, "range": "100 m"}, "OUT_OF_DOMAIN", "/elevation"))
    out.append(refusal(len(out) + 1, {"azimuth": 10, "elevation": 5, "range": "-1 m"}, "OUT_OF_DOMAIN", "/range"))
    return out


def enu_to_aer_vectors():
    rng = random.Random(66202)
    cases = [(1200.0, -900.0, 150.0), (0.0, 100.0, 0.0), (100.0, 0.0, 0.0), (-100.0, 0.0, 100.0), (3.0, 4.0, 12.0), (-250.0, -250.0, -40.0)]
    while len(cases) < 20:
        cases.append(tuple(round(rng.uniform(-50000, 50000), 3) for _ in range(3)))
    out = []
    for i, (east, north, up) in enumerate(cases, 1):
        horizontal = math.hypot(east, north)
        out.append(vec(i, {"east": f"{east} m", "north": f"{north} m", "up": f"{up} m"},
                       {"result.azimuth.value": math.degrees(math.atan2(east, north)) % 360, "result.elevation.value": math.degrees(math.atan2(up, horizontal)),
                        "result.range.value": math.sqrt(east * east + north * north + up * up), "result.horizontal.value": horizontal}, TRIG, {"abs": 1e-7}))
    out.append(refusal(len(out) + 1, {"east": "1e9 m", "north": "0 m", "up": "0 m"}, "OUT_OF_DOMAIN", "/east"))
    out.append(refusal(len(out) + 1, {"east": "10 m", "north": "0 m", "up": "-2e8 m"}, "OUT_OF_DOMAIN", "/up"))
    return out


def enu_ned_vectors():
    rng = random.Random(66203)
    out = []
    cases = [("ned", 300.0, -120.0, 45.0)]
    while len(cases) < 20:
        cases.append((["enu", "ned"][len(cases) % 2], *(round(rng.uniform(-9000, 9000), 3) for _ in range(3))))
    for i, (frame, north, east, up) in enumerate(cases, 1):
        inp = {"north": f"{north} m", "east": f"{east} m"}
        if frame == "ned":
            inp.update({"frame": "ned", "down": f"{-up} m"})
        else:
            inp["up"] = f"{up} m"
            if i % 3 == 0:
                inp["frame"] = "enu"
        out.append(vec(i, inp, {"result.north.value": north, "result.east.value": east, "result.down.value": -up, "result.up.value": up}, TRIG, {"abs": 1e-9}))
    out.append(refusal(len(out) + 1, {"frame": "ned", "north": "1 m", "east": "2 m", "up": "3 m"}, field="/down"))
    out.append(refusal(len(out) + 1, {"north": "1 m", "east": "2 m", "down": "3 m"}, field="/up"))
    out.append(refusal(len(out) + 1, {"north": "1 m", "east": "2 m", "up": "3 m", "down": "3 m"}, field="/down"))
    return out


def rotation_vectors():
    rng = random.Random(66204)
    cases = [(35.0, 139.0, "ned"), (0.0, 0.0, "enu"), (90.0, 0.0, "enu"), (0.0, 90.0, "ned"), (-45.0, -90.0, "enu"), (40.446111, -79.982222, "enu")]
    while len(cases) < 20:
        cases.append((round(rng.uniform(-89, 89), 5), round(rng.uniform(-179, 179), 5), ["enu", "ned"][len(cases) % 2]))
    out = []
    for i, (lat, lon, frame) in enumerate(cases, 1):
        p, l = math.radians(lat), math.radians(lon)
        east = (-math.sin(l), math.cos(l), 0.0)
        north = (-math.sin(p) * math.cos(l), -math.sin(p) * math.sin(l), math.cos(p))
        up = (math.cos(p) * math.cos(l), math.cos(p) * math.sin(l), math.sin(p))
        rows = [north, east, tuple(-v for v in up)] if frame == "ned" else [east, north, up]
        inp = {"lat0": lat, "lon0": lon}
        if frame == "ned" or i % 3 == 0:
            inp["frame"] = frame
        exp = {f"result.rotation.{r}.{axis}": float(rows[r][c]) for r in range(3) for c, axis in enumerate("xyz")}
        out.append(vec(i, inp, exp, TRIG, {"abs": 1e-14}))
    out.append(refusal(len(out) + 1, {"lat0": 95, "lon0": 0}))
    return out


# ---------------------------------------------------------------- UTM convergence and scale

TM_EXACT = ("GeographicLib TransverseMercatorProj -t (the exact transverse Mercator, by elliptic functions) at k0 = 0.9996 (tools/vectors/gen_geodesy_focused.py)", GEOD_VER)


def tm_exact(lat, lon, zone, a=6378137.0, rf=298.257223563):
    """(convergence in degrees, scale) from GeographicLib's exact transverse Mercator, not the Krüger series the core uses."""
    cm = 6 * zone - 183
    out = subprocess.run(["TransverseMercatorProj", "-t", "-l", f"{cm}", "-k", "0.9996", "-p", "10", "-e", f"{a:.6f}", f"{1 / rf:.15f}"],
                         input=f"{lat:.12f} {lon:.12f}\n", capture_output=True, text=True, check=True).stdout.split()
    return float(out[2]), float(out[3])


def grid_figure_vectors(first):
    rng = random.Random({"convergence": 66205, "scale": 66206}[first])
    # (lat, lon, forced zone or None, ellipsoid or None); none in the Norway and Svalbard exception bands unless forced.
    cases = [(78.2232, 15.6267, 33, None) if first == "convergence" else (60.3913, 5.3221, 32, None),
             (40.446111, -79.982222, None, None), (0.0, -177.0, None, None), (-33.8688, 151.2093, None, None), (45.0, 9.0, None, None),
             (51.4778, -0.0015, None, None), (51.4778, -0.0015, 31, None), (35.6762, 139.6503, None, "grs80"), (19.4326, -99.1332, None, "clarke1866")]
    while len(cases) < 20:
        cases.append((round(rng.uniform(-79, 55), 5), round(rng.uniform(-179.9, 179.9), 5), None, None))
    out = []
    for i, (lat, lon, zone, name) in enumerate(cases, 1):
        standard = int((lon + 180) // 6) % 60 + 1
        a, rf = ELLIPSOIDS[name]
        used = zone or standard
        convergence, scale = tm_exact(lat, lon, used, a, rf)
        inp = {"lat": lat, "lon": lon}
        if zone:
            inp["zone"] = zone
        if name:
            inp["ellipsoid"] = name
        out.append(vec(i, inp, {"result.convergence.value": convergence, "result.scale": scale, "result.zone": float(used),
                                "result.central_meridian.value": float(6 * used - 183)}, TM_EXACT, {"abs": 1e-9}))
    out.append(refusal(len(out) + 1, {"lat": 85, "lon": 10}, "OUT_OF_DOMAIN", "/lat"))
    out.append(refusal(len(out) + 1, {"lat": 40, "lon": -80, "zone": 30}, field="/zone"))
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    selected = set(sys.argv[2:])
    files = {"geodesy.parse.format-dd": lambda: format_vectors("dd"), "geodesy.parse.format-dms": lambda: format_vectors("dms"),
             "geodesy.parse.format-ddm": lambda: format_vectors("ddm"), "geodesy.parse.parse-dms": lambda: parse_vectors("dms"),
             "geodesy.parse.parse-ddm": lambda: parse_vectors("ddm"), "geodesy.magnetic.magnetic-to-true": magnetic_vectors,
             "geodesy.ellipsoid.meridian-arc": arc_vectors, "geodesy.ellipsoid.degree-lengths": degree_vectors,
             "geodesy.frame.aer-to-enu": aer_to_enu_vectors, "geodesy.frame.enu-to-aer": enu_to_aer_vectors,
             "geodesy.frame.enu-ned": enu_ned_vectors, "geodesy.frame.local-rotation-matrix": rotation_vectors,
             "geodesy.projection.convergence": lambda: grid_figure_vectors("convergence"),
             "geodesy.projection.scale-factor": lambda: grid_figure_vectors("scale")}
    for tool, make in files.items():
        if selected and tool not in selected:
            continue
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in make()))


if __name__ == "__main__":
    main()
