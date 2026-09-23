#!/usr/bin/env python3
"""Generates golden vectors for the units domain (core/vectors/units.*.jsonl).

Expected values are computed independently of the Rust core: each unit is
restated here from its published exact definition (NIST SP 811 Appendix B,
NIST Handbook 44 Appendix C), and each conversion is done in exact rational
arithmetic, then rounded once to binary64. Units involving pi use Python's
math.pi and carry a looser tolerance.

Vectors are immutable once published (verification spec). Rerunning this script
must reproduce the committed files byte for byte; the vector lint checks that.
"""
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

SRC = "NIST SP 811 (2008) Appendix B.8 and NIST Handbook 44 Appendix C exact definitions; expected value by exact rational arithmetic (tools/vectors/gen_units.py)"
VER = "SP 811 2008; HB44 2026"
REL_EXACT = 5e-16  # one conversion is at most two roundings of an exact ratio
REL_PI = 2e-15

FT = F(3048, 10000)
IN = FT / 12
MI = 5280 * FT
LB = F(45359237, 10**8)
G0 = F(980665, 10**5)
GAL = 231 * IN**3

LINEAR = {
    "length": {"m": 1, "km": 1000, "cm": F(1, 100), "mm": F(1, 1000), "ft": FT, "ftUS": F(1200, 3937),
               "in": IN, "yd": 3 * FT, "mi": MI, "NM": 1852},
    "area": {"m2": 1, "km2": 10**6, "ha": 10**4, "ac": 43560 * FT**2, "ft2": FT**2, "mi2": MI**2},
    "volume": {"m3": 1, "L": F(1, 1000), "mL": F(1, 10**6), "galUS": GAL, "galImp": F(454609, 10**8),
               "ft3": FT**3, "yd3": (3 * FT)**3},
    "mass": {"kg": 1, "g": F(1, 1000), "lb": LB, "oz": LB / 16, "t": 1000},
    "speed": {"m/s": 1, "km/h": F(1000, 3600), "kt": F(1852, 3600), "mph": MI / 3600, "ft/s": FT},
    "vertical-speed": {"m/s": 1, "ft/min": FT / 60, "m/min": F(1, 60), "ft/s": FT},
    "acceleration": {"m/s2": 1, "g0": G0, "ft/s2": FT},
    "pressure": {"Pa": 1, "hPa": 100, "kPa": 1000, "mbar": 100, "bar": 10**5, "inHg": F(3386389, 1000),
                 "mmHg": F(133322387415, 10**9), "psi": LB * G0 / IN**2, "atm": 101325},
    "temperature-difference": {"K": 1, "degC": 1, "degF": F(5, 9)},
    "angle": {"deg": 1, "gon": F(9, 10), "arcmin": F(1, 60), "arcsec": F(1, 3600), "turn": 360,
              "mil-nato": F(360, 6400), "mil-warsaw": F(360, 6000), "mil-sweden": F(360, 6300)},
    "angular-rate": {"deg/s": 1, "deg/min": F(1, 60), "rpm": 6},
    "time": {"s": 1, "ms": F(1, 1000), "min": 60, "h": 3600, "d": 86400},
    "energy": {"J": 1, "kJ": 1000, "MJ": 10**6, "Wh": 3600, "kWh": 3600000},
    "power": {"W": 1, "kW": 1000, "hp": 550 * FT * LB * G0},
    "density": {"kg/m3": 1, "g/cm3": 1000, "lb/galUS": LB / GAL, "lb/ft3": LB / FT**3},
    "frequency": {"Hz": 1, "kHz": 1000, "MHz": 10**6, "GHz": 10**9},
    "data-rate": {"bit/s": 1, "kbit/s": 1000, "Mbit/s": 10**6, "Gbit/s": 10**9},
    "charge": {"C": 1, "mAh": F(36, 10), "Ah": 3600},
}
PI_UNITS = {"angle": {"rad": 180 / math.pi, "mrad": 0.18 / math.pi}, "angular-rate": {"rad/s": 180 / math.pi}}
TEMP = {"K": (1, 0), "degC": (1, F(27315, 100)), "degF": (F(5, 9), F(45967, 100))}  # K = (x + off) * f


def dec(s):
    return F(s)


def lin(group, x, a, b):
    table = LINEAR[group]
    if a in table and b in table:
        return float(dec(x) * F(table[a]) / F(table[b])), REL_EXACT
    fa = PI_UNITS.get(group, {}).get(a) or float(table[a])
    fb = PI_UNITS.get(group, {}).get(b) or float(table[b])
    return float(dec(x)) * fa / fb, REL_PI


def temp(x, a, b):
    fa, oa = TEMP[a]
    fb, ob = TEMP[b]
    return float((dec(x) + oa) * fa / fb - ob), REL_EXACT


def vec(i, input_, expect, tol=None, src=SRC, ver=VER):
    v = {"id": f"v{i:03d}", "input": input_, "expect": expect, "source": src, "sourceVersion": ver}
    if tol:
        v["tolerance"] = tol
    return v


def convert_vectors(group, cases, fn):
    out = []
    for i, (x, a, b) in enumerate(cases, 1):
        want, rel = fn(x, a, b)
        out.append(vec(i, {"value": f"{x} {a}", "to": b},
                       {"ok": True, "result.converted.value": want, "result.converted.unit": b},
                       {"result.converted.value": {"rel": rel}}))
    return out


CASES = {
    "length": [("5280", "ft", "m"), ("1", "NM", "m"), ("1000000", "ftUS", "m"), ("1000000", "ft", "m"),
               ("100", "NM", "mi"), ("120", "m", "ft"), ("12.5", "in", "mm"), ("0", "km", "NM"), ("-3.5", "yd", "ft")],
    "area": [("1", "ac", "ft2"), ("40", "ac", "ha"), ("43560", "ft2", "ac"), ("1", "mi2", "ac"), ("2.5", "km2", "ha")],
    "volume": [("50", "galUS", "L"), ("1", "galUS", "ft3"), ("100", "L", "galUS"),
               ("1", "galImp", "galUS"), ("27", "ft3", "yd3")],
    "mass": [("2550", "lb", "kg"), ("25", "kg", "lb"), ("16", "oz", "lb"), ("1", "t", "lb"), ("0.5", "kg", "g")],
    "speed": [("100", "kt", "mph"), ("120", "mph", "kt"), ("100", "kt", "km/h"), ("10", "m/s", "kt"),
              ("1", "kt", "m/s"), ("88", "ft/s", "mph")],
    "vertical-speed": [("500", "ft/min", "m/s"), ("5", "m/s", "ft/min"), ("1000", "ft/min", "m/min"),
                       ("-700", "ft/min", "m/s"), ("60", "ft/s", "ft/min")],
    "acceleration": [("2", "g0", "m/s2"), ("1", "g0", "ft/s2"), ("9.80665", "m/s2", "g0"),
                     ("32.174", "ft/s2", "g0"), ("-1.5", "g0", "m/s2")],
    "pressure": [("29.92", "inHg", "hPa"), ("1013.25", "hPa", "inHg"), ("1", "psi", "Pa"), ("1", "atm", "inHg"),
                 ("760", "mmHg", "hPa"), ("30", "psi", "kPa"), ("1013", "mbar", "hPa")],
    "temperature-difference": [("18", "degF", "K"), ("1", "degC", "degF"), ("-9", "degF", "degC"),
                               ("10", "K", "degF"), ("0", "degC", "K")],
    "angle": [("1600", "mil-nato", "deg"), ("100", "gon", "deg"), ("90", "deg", "arcmin"), ("1", "deg", "arcsec"),
              ("180", "deg", "rad"), ("1", "rad", "deg"), ("0.5", "turn", "mil-warsaw"), ("1", "mrad", "arcmin")],
    "angular-rate": [("3", "deg/s", "rpm"), ("360", "deg/min", "deg/s"), ("1", "rpm", "deg/s"),
                     ("3", "deg/s", "rad/s"), ("1", "rad/s", "deg/s")],
    "time": [("90", "min", "h"), ("1", "d", "s"), ("1500", "ms", "s"), ("2.5", "h", "min"), ("86400", "s", "d")],
    "energy": [("99", "Wh", "kJ"), ("1", "kWh", "MJ"), ("3600", "J", "Wh"), ("100", "Wh", "J"), ("0", "kJ", "Wh")],
    "power": [("180", "hp", "kW"), ("1", "hp", "W"), ("100", "kW", "hp"), ("750", "W", "hp"), ("0", "W", "kW")],
    "density": [("6.7", "lb/galUS", "g/cm3"), ("800", "kg/m3", "lb/galUS"), ("1", "g/cm3", "lb/ft3"),
                ("62.4", "lb/ft3", "kg/m3"), ("6", "lb/galUS", "kg/m3")],
    "frequency": [("2.4", "GHz", "MHz"), ("5800", "MHz", "GHz"), ("121.5", "MHz", "kHz"), ("1", "Hz", "kHz"),
                  ("433", "MHz", "Hz")],
    "data-rate": [("20", "Mbit/s", "kbit/s"), ("1", "Gbit/s", "Mbit/s"), ("56", "kbit/s", "bit/s"),
                  ("1500", "kbit/s", "Mbit/s"), ("0.5", "Gbit/s", "bit/s")],
    "charge": [("5000", "mAh", "Ah"), ("1", "Ah", "C"), ("3.6", "C", "mAh"), ("2200", "mAh", "C"),
               ("10", "Ah", "mAh")],
}

TEMP_CASES = [("30", "degC", "degF"), ("32", "degF", "K"), ("100", "degC", "degF"), ("-40", "degF", "degC"),
              ("0", "K", "degC"), ("15", "degC", "K"), ("59", "degF", "degC")]

# The hand-picked cases above are the ones worth reading: round numbers, the
# pairs people actually convert, the signs and zeroes that catch an offset
# dropped. They are too few to promote a tool, and choosing more by hand would
# be choosing which mistakes to look for. So every ordered pair of units in a
# group is also swept, at magnitudes chosen to exercise a factor in both
# directions. A factor that is wrong is wrong at any magnitude; one applied
# upside down, or an offset lost, is not.
#
# The sweep is APPENDED, so every vector published before it keeps its id and
# its value and the file stays reproducible.
SWEEP_VALUES = ["1", "2.5", "100", "0.125", "7.5", "-3", "60", "1000"]


def sweep(group, want):
    """Ordered pairs of every unit in the group, cycling magnitudes."""
    units = sorted(set(LINEAR.get(group, {})) | set(PI_UNITS.get(group, {})))
    pairs = [(a, b) for a in units for b in units if a != b]
    if not pairs:
        return []
    return [
        (SWEEP_VALUES[(k // len(pairs) + k) % len(SWEEP_VALUES)],) + pairs[k % len(pairs)]
        for k in range(want)
    ]

# (group, slug, from, to, examples) for allow-listed pairs; must match core/crates/gp-units/src/pairs.rs
PAIRS = [
    ("speed", "kt-to-mph", "kt", "mph", ["100", "1", "250"]),
    ("speed", "mph-to-kt", "mph", "kt", ["120", "1", "65"]),
    ("speed", "kt-to-kmh", "kt", "km/h", ["100", "1", "35"]),
    ("speed", "kmh-to-kt", "km/h", "kt", ["50", "1", "100"]),
    ("speed", "mps-to-kt", "m/s", "kt", ["10", "1", "7.5"]),
    ("speed", "kt-to-mps", "kt", "m/s", ["20", "1", "15"]),
    ("length", "ft-to-m", "ft", "m", ["5280", "1", "400"]),
    ("length", "m-to-ft", "m", "ft", ["120", "1", "0.3048"]),
    ("length", "nm-to-km", "NM", "km", ["100", "1", "60"]),
    ("length", "km-to-nm", "km", "NM", ["100", "1", "1.852"]),
    ("length", "nm-to-mi", "NM", "mi", ["100", "1", "25"]),
    ("length", "mi-to-nm", "mi", "NM", ["100", "1", "3"]),
    ("length", "mi-to-km", "mi", "km", ["10", "1", "26.2"]),
    ("length", "km-to-mi", "km", "mi", ["10", "1", "42.195"]),
    ("length", "ftus-to-m", "ftUS", "m", ["1000000", "1", "3937"]),
    ("length", "m-to-ftus", "m", "ftUS", ["304800.6096", "1", "1200"]),
    ("length", "ftus-to-ft", "ftUS", "ft", ["1000000", "1", "2000000"]),
    ("length", "in-to-mm", "in", "mm", ["1", "12.5", "0.25"]),
    ("length", "mm-to-in", "mm", "in", ["25.4", "1", "100"]),
    ("pressure", "inhg-to-hpa", "inHg", "hPa", ["29.92", "30.12", "28.5"]),
    ("pressure", "hpa-to-inhg", "hPa", "inHg", ["1013.25", "1000", "990"]),
    ("pressure", "psi-to-kpa", "psi", "kPa", ["30", "1", "1850"]),
    ("pressure", "kpa-to-psi", "kPa", "psi", ["200", "1", "101.325"]),
    ("temperature", "c-to-f", "degC", "degF", ["15", "-40", "37"]),
    ("temperature", "f-to-c", "degF", "degC", ["59", "32", "-4"]),
    ("volume", "gal-to-l", "galUS", "L", ["50", "1", "12.5"]),
    ("volume", "l-to-gal", "L", "galUS", ["100", "1", "3.785411784"]),
    ("mass", "lb-to-kg", "lb", "kg", ["2550", "1", "55"]),
    ("mass", "kg-to-lb", "kg", "lb", ["25", "1", "0.25"]),
    ("area", "ac-to-ha", "ac", "ha", ["40", "1", "640"]),
    ("area", "ha-to-ac", "ha", "ac", ["10", "1", "0.5"]),
    ("area", "ft2-to-ac", "ft2", "ac", ["43560", "10000", "1"]),
    ("vertical-speed", "fpm-to-mps", "ft/min", "m/s", ["500", "1", "-1000"]),
    ("angle", "deg-to-rad", "deg", "rad", ["180", "1", "90"]),
    ("angle", "rad-to-deg", "rad", "deg", ["1", "3.141592653589793", "0.5"]),
]


TARGET_VECTORS = 22
for _group, _cases in CASES.items():
    _cases.extend(sweep(_group, max(0, TARGET_VECTORS - len(_cases))))
TEMP_CASES.extend(
    (v, a, b)
    for k, (v, (a, b)) in enumerate(
        zip(
            (SWEEP_VALUES[(i // 6 + i) % len(SWEEP_VALUES)] for i in range(TARGET_VECTORS - len(TEMP_CASES))),
            [(a, b) for a in sorted(TEMP) for b in sorted(TEMP) if a != b] * 4,
        )
    )
)


def special_vectors():
    """Fuel, slope, and normalize: expected values derived by hand from the definitions."""
    lb_gal = F(6)
    jet = F(67, 10)
    fuel = [
        ({"volume": "50 galUS", "fuel": "avgas-100ll"}, {"result.mass.value": float(50 * lb_gal), "result.mass.unit": "lb",
                                                        "meta.warnings.*.code": "NOMINAL_VALUE_USED"}),
        ({"mass": "300 lb", "fuel": "avgas-100ll"}, {"result.volume.value": 50.0, "result.volume.unit": "galUS"}),
        ({"volume": "100 galUS", "fuel": "jet-a"}, {"result.mass.value": float(100 * jet), "result.mass.unit": "lb"}),
        ({"volume": "20 galUS", "density": "6.02 lb/galUS"}, {"result.mass.value": float(20 * F("6.02"))}),
        ({"volume": "100 L", "density": "0.8 g/cm3"}, {"result.mass.value": float(F(80) / LB), "result.mass.unit": "lb"}),
        ({"volume": "50 galUS"}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/density"}),
    ]
    slope = [
        ({"value": 8.333333333333334, "from": "percent"}, {"result.degrees.value": math.degrees(math.atan(1 / 12)),
                                                           "result.ratio.value": float(F(8.333333333333334) / 100)}),
        ({"value": 45, "from": "degrees"}, {"result.ratio.value": 1.0, "result.percent.value": 100.0}),
        ({"value": 0.5, "from": "ratio"}, {"result.percent.value": 50.0, "result.permille.value": 500.0}),
        ({"value": 25, "from": "permille"}, {"result.ratio.value": 0.025}),
        ({"value": -6, "from": "percent"}, {"result.degrees.value": math.degrees(math.atan(-0.06))}),
        ({"value": 90, "from": "degrees"}, {"ok": False, "error.code": "OUT_OF_DOMAIN"}),
    ]
    norm = [
        ({"value": "145 kts", "quantity": "speed"}, {"result.normalized.value": float(145 * F(1852, 3600)),
                                                     "result.normalized.unit": "m/s"}),
        ({"value": "12 nm", "quantity": "distance"}, {"result.normalized.value": 22224.0,
                                                      "meta.warnings.*.code": "UNIT_ASSUMED"}),
        ({"value": "29.92 inHg", "quantity": "pressure"}, {"result.normalized.value": float(F("29.92") * F(3386389, 1000))}),
        ({"value": "15 C", "quantity": "temperature"}, {"result.normalized.value": 288.15, "result.normalized.unit": "K"}),
        ({"value": "5280'", "quantity": "length"}, {"result.normalized.value": 1609.344}),
        ({"value": "1600 mil", "quantity": "angle"}, {"ok": False, "error.code": "UNIT_MISMATCH"}),
    ]
    out = {}
    sources = {
        "units.fuel.convert": ("FAA-H-8083-25C Chapter 10 nominal fuel weights (avgas 6 lb/gal, jet fuel 6.7 lb/gal); mass = volume x density by exact rational arithmetic", "FAA-H-8083-25C (2023)"),
        "units.slope.convert": ("Definition of grade: ratio = rise/run = tan(angle); angles from Python math.atan", "definition"),
        "units.quantity.normalize": (SRC, VER),
    }
    for tool, cases, rel in [("units.fuel.convert", fuel, REL_EXACT), ("units.slope.convert", slope, 2e-15),
                             ("units.quantity.normalize", norm, REL_EXACT)]:
        vs = []
        for i, (inp, exp) in enumerate(cases, 1):
            exp = dict(exp)
            exp.setdefault("ok", True)
            tol = {k: {"rel": rel} for k, v in exp.items() if isinstance(v, float)}
            vs.append(vec(i, inp, exp, tol or None, *sources[tool]))
        out[tool] = vs
    return out


def write(path, vectors):
    text = "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vectors)
    path.write_text(text)


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    out.mkdir(parents=True, exist_ok=True)
    for group, cases in CASES.items():
        write(out / f"units.{group}.convert.jsonl", convert_vectors(group, cases, lambda x, a, b, g=group: lin(g, x, a, b)))
    write(out / "units.temperature.convert.jsonl", convert_vectors("temperature", TEMP_CASES, temp))
    for group, slug, a, b, xs in PAIRS:
        fn = temp if group == "temperature" else (lambda x, a, b, g=group: lin(g, x, a, b))
        vs = []
        for i, x in enumerate(xs, 1):
            want, rel = fn(x, a, b)
            vs.append(vec(i, {"value": float(x)},
                          {"ok": True, "result.converted.value": want, "result.converted.unit": b},
                          {"result.converted.value": {"rel": rel}}))
        write(out / f"units.{group}.{slug}.jsonl", vs)
    for tool, vs in special_vectors().items():
        write(out / f"{tool}.jsonl", vs)


if __name__ == "__main__":
    main()
