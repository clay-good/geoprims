#!/usr/bin/env python3
"""Golden vectors for aviation.ifr.circling-radius (add-aviation-practical-
additions, aviation/approach-references), worked in Python from Order
8260.3G paragraph 2-7-1.a: Formula 2-7-1,
V_KTAS = V_KIAS x 171233 x sqrt(303 - 0.00198 (elev + k)) / (288 - 0.00198 (elev + k))^2.628
with k >= 1,000 ft, and Formula 2-7-2,
CAR = 2 (V_KTAS + 25)^2 / (tan(bank) x 68625.4) + S, at least 1.30 NM,
with Table 2-7-1's speed, bank, and straight segment. The first vector is
the Instrument Procedures Handbook's printed example (FAA-H-8083-16B,
chapter 4): Category C at a sea-level airport with a 1,500 ft HAA, 2.86 NM.

    gen_circling.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import json
import math

TABLE = {"A": (90, 25, 0.4, 1.3), "B": (120, 25, 0.4, 1.5), "C": (140, 20, 0.5, 1.7),
         "D": (165, 20, 0.6, 2.3), "E": (200, 22, 0.7, 4.5)}
SRC = "Order 8260.3G Formulas 2-7-1 and 2-7-2, worked in Python (tools/vectors/gen_circling.py)"
VER = "Order 8260.3G, effective 07/01/2024"


def solve(cat, elev, circ=None):
    ias, bank, s, legacy = TABLE[cat]
    k = 1000.0 if circ is None else max(1000.0, circ - elev)
    h = 0.00198 * (elev + k)
    tas = ias * 171233 * math.sqrt(303 - h) / (288 - h) ** 2.628
    car = 2 * (tas + 25) ** 2 / (math.tan(math.radians(bank)) * 68625.4) + s
    return {"result.radius.value": max(car, 1.30), "result.radius_formula.value": car,
            "result.tas.value": tas, "result.height_used.value": k, "result.legacy_radius.value": legacy}


vectors = []


def add(inp, exp, source=SRC, tol=None):
    vectors.append({"id": f"v{len(vectors) + 1:03d}", "input": inp, "expect": {**exp, "ok": True},
                    "source": source, "sourceVersion": VER,
                    "tolerance": tol if tol is not None else {k: {"rel": 1e-9, "abs": 1e-9} for k, v in exp.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}})


add({"category": "C", "airport_elevation": "0 ft", "circling_altitude": "1500 ft"}, {"result.radius.value": 2.86},
    source="FAA Instrument Procedures Handbook (FAA-H-8083-16B) chapter 4, printed to two decimals",
    tol={"result.radius.value": {"abs": 0.005}})
for elev in (0, 2000, 5000):
    for cat in "ABCDE":
        add({"category": cat, "airport_elevation": f"{elev} ft"}, solve(cat, elev))
for cat, elev, circ in (("C", 0, 1000), ("D", 1000, 3200), ("B", 6500, 7300), ("E", 300, 2100)):
    add({"category": cat, "airport_elevation": f"{elev} ft", "circling_altitude": f"{circ} ft"}, solve(cat, elev, circ))
for inp, field in (({"category": "C", "airport_elevation": "1000 ft", "circling_altitude": "900 ft"}, "/circling_altitude"),
                   ({"category": "A", "airport_elevation": "16000 ft"}, "/airport_elevation")):
    vectors.append({"id": f"v{len(vectors) + 1:03d}", "input": inp,
                    "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN", "error.field": field},
                    "source": SRC, "sourceVersion": VER, "tolerance": {}})
# The TPP legend's circling table (d-TPP cycle 2611, 29 OCT 2026 to 26 NOV
# 2026), as printed, one vector per band and category, at an MDA inside the
# band. Appended after the published vectors.
LEGEND = [((0, 800), [1.3, 1.7, 2.7, 3.6, 4.5]), ((500, 2000), [1.3, 1.8, 2.8, 3.7, 4.6]),
          ((2000, 4000), [1.3, 1.8, 2.9, 3.8, 4.8]), ((4500, 6000), [1.3, 1.9, 3.0, 4.0, 5.0]),
          ((5500, 8000), [1.4, 2.0, 3.2, 4.2, 5.3]), ((7000, 10000), [1.4, 2.1, 3.3, 4.4, 5.5])]
for (elev, mda), row in LEGEND:
    for cat, printed in zip("ABCDE", row):
        add({"category": cat, "airport_elevation": f"{elev} ft", "circling_altitude": f"{mda} ft"},
            {"result.legend_radius.value": printed},
            source="FAA Terminal Procedures Publication legend (d-TPP cycle 2611), printed cell",
            tol={"result.legend_radius.value": {"abs": 1e-9}})
for v in vectors:
    print(json.dumps(v, ensure_ascii=False, separators=(",", ":")))
