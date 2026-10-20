#!/usr/bin/env python3
"""Golden vectors for aviation.loading.fuel-density-temperature
(add-aviation-practical-additions, aviation/fuel-density).

The first vectors are cells printed in Measurement Canada's volume correction
factor tables (issued January 2016): Jet A at 800 kg/m3 and aviation gasoline
at 710 kg/m3 (API MPMS 11.1 Table 54B), and Jet B at 760 kg/m3 (Table 54A),
held to half the table's last digit plus the standard's intermediate
rounding, 6e-5. The rest are worked in Python from the same formula,
VCF = exp(-a dT (1 + 0.8 a dT)), with the metric constants of the 1980
tables, or from Chevron's single coefficient, 1 / (1 + 0.00099 dT).

    gen_fuel_density.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import json
import math

TOOL = "aviation.loading.fuel-density-temperature"
MC = "Measurement Canada VCF table (January 2016), printed cell"
MC_VER = "tables issued January 2016"
PY = "Worked in Python from the API MPMS 11.1 (1980) metric constants (tools/vectors/gen_fuel_density.py)"
PY_VER = "CM/eXL/FR-EN Rev. B, October 2017"
LB = 0.45359237
GAL = 3.785411784e-3


def alpha(rho, crude=False):
    if crude:
        return 613.9723 / rho**2
    if rho < 770.5:
        return 346.4228 / rho**2 + 0.4388 / rho
    if rho < 787.5:
        return -0.00336312 + 2680.3206 / rho**2
    if rho < 838.5:
        return 594.5418 / rho**2
    return 186.9696 / rho**2 + 0.4862 / rho


def vcf(rho, t, crude=False):
    a = alpha(rho, crude)
    d = t - 15
    return math.exp(-a * d * (1 + 0.8 * a * d))


def linear(t, beta=0.00099):
    return 1 / (1 + beta * (t - 15))


def tol(exp, rel=1e-9):
    return {k: {"rel": rel, "abs": 1e-9} for k, v in exp.items() if isinstance(v, float)}


vectors = []


def add(inp, exp, source=PY, version=PY_VER, tolerance=None):
    vectors.append({
        "id": f"v{len(vectors) + 1:03d}",
        "input": inp,
        "expect": {**exp, "ok": True},
        "source": source,
        "sourceVersion": version,
        "tolerance": tolerance if tolerance is not None else tol(exp),
    })


def bad(inp, code, field):
    vectors.append({
        "id": f"v{len(vectors) + 1:03d}",
        "input": inp,
        "expect": {"ok": False, "error.code": code, "error.field": field},
        "source": PY,
        "sourceVersion": PY_VER,
        "tolerance": {},
    })


def cell(fuel, rho, t, printed, name):
    add({"fuel": fuel, "density_15": f"{rho} kg/m3", "temperature": f"{t} degC"},
        {"result.vcf": printed}, source=f"{MC}: {name} at {t} °C", version=MC_VER,
        tolerance={"result.vcf": {"abs": 6e-5}})


# The primary example first, then the design's seven Jet A points.
cell("jet-a", 800, -20, 1.0322, "Jet A, 800 kg/m³ (54B)")
for t, v in [(-40, 1.0502), (0, 1.0139), (10, 1.0046), (20, 0.9953), (30, 0.9860), (40, 0.9766)]:
    cell("jet-a", 800, t, v, "Jet A, 800 kg/m³ (54B)")
# Cells where the formula sits on a rounding half-step of the table.
cell("jet-a", 800, -28.8, 1.0402, "Jet A, 800 kg/m³ (54B)")
for t, v in [(-20, 1.0450), (-38.5, 1.0681), (25, 0.9869)]:
    cell("100ll", 710, t, v, "aviation gasoline, 710 kg/m³ (54B)")
for t, v in [(-32.7, 1.0499), (30, 0.9840)]:
    cell("wide-cut", 760, t, v, "Jet B, 760 kg/m³ (54A)")

# Typical densities (Chevron Figure 2.1) through the formula.
for fuel, rho, t, crude in [("jet-a", 810.0, -10.0, False), ("100ll", 715.0, 35.0, False), ("wide-cut", 762.0, -30.0, True)]:
    v = vcf(rho, t, crude)
    add({"fuel": fuel, "temperature": f"{t:g} degC"},
        {"result.vcf": v, "result.density.value": rho * v, "result.density_15.value": rho})
# Entered densities in the transition and fuel-oil zones of Table 54B.
for rho, t in [(780.0, 45.0), (845.0, -15.0)]:
    v = vcf(rho, t)
    add({"density_15": f"{rho:g} kg/m3", "temperature": f"{t:g} degC"},
        {"result.vcf": v, "result.density.value": rho * v})
# A volume's weight and its volume at 15 °C.
v = vcf(804.0, 32.0)
add({"fuel": "jet-a", "density_15": "804 kg/m3", "temperature": "32 degC", "volume": "100 gal"},
    {"result.weight.value": 100 * GAL * 804 * v / LB, "result.volume_15.value": 100 * v})
# Chevron's coefficient: the linear method, and its own example's two ends.
for t in [0.0, 40.0]:
    lv = linear(t)
    add({"fuel": "jet-a", "temperature": f"{t:g} degC", "method": "linear"},
        {"result.vcf": lv, "result.density.value": 810 * lv, "result.vcf_table": vcf(810, t)},
        source="Chevron FTR-3 (2007) chapter 2, 0.00099 per °C, worked in Python", version="FTR-3, 2007")
# An entered coefficient for avgas.
add({"fuel": "100ll", "temperature": "-5 degC", "method": "linear", "expansion": 0.0012},
    {"result.vcf": linear(-5, 0.0012), "result.density.value": 715 * linear(-5, 0.0012)})
# Refusals.
bad({"fuel": "jet-a", "temperature": "-51 degC"}, "OUT_OF_DOMAIN", "/temperature")
bad({"fuel": "jet-a", "temperature": "61 degC"}, "OUT_OF_DOMAIN", "/temperature")
bad({"density_15": "640 kg/m3", "temperature": "0 degC"}, "OUT_OF_DOMAIN", "/density_15")
bad({"fuel": "100ll", "temperature": "0 degC", "method": "linear"}, "INVALID_INPUT", "/expansion")

for v in vectors:
    print(json.dumps(v, ensure_ascii=False, separators=(",", ":")))
