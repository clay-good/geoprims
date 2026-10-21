#!/usr/bin/env python3
"""Golden vectors for aviation.ifr.visibility-to-cmv (add-aviation-practical-
additions, aviation/approach-references, the EASA half): AMC10
CAT.OP.MPA.110 Table 16 as printed in EASA's Easy Access Rules for Air
Operations, Revision 24 (March 2026), read 2026-10-20: CMV = reported
visibility x 1.5 (day) or 2.0 (night) with high-intensity approach and
runway lights, 1.0 or 1.5 with other lights, 1.0 by day with none and not
applicable at night; 1.0 for flight planning; never for take-off minima or
to continue an approach in LVOs.

    gen_cmv.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import json

FACTOR = {("hi", "day"): 1.5, ("hi", "night"): 2.0, ("other", "day"): 1.0, ("other", "night"): 1.5, ("none", "day"): 1.0}
SRC = "EASA Easy Access Rules for Air Operations, AMC10 CAT.OP.MPA.110 Table 16, as printed"
VER = "Revision 24, March 2026"
MI = 1609.344
vectors = []


def add(inp, cmv, factor):
    vectors.append({"id": f"v{len(vectors) + 1:03d}", "input": inp,
                    "expect": {"ok": True, "result.cmv.value": cmv, "result.factor": factor},
                    "source": SRC, "sourceVersion": VER,
                    "tolerance": {"result.cmv.value": {"rel": 1e-12, "abs": 1e-9}, "result.factor": {"abs": 1e-12}}})


def bad(inp, field):
    vectors.append({"id": f"v{len(vectors) + 1:03d}", "input": inp,
                    "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN", "error.field": field},
                    "source": SRC, "sourceVersion": VER, "tolerance": {}})


add({"visibility": "1500 m", "lights": "hi", "period": "night"}, 3000.0, 2.0)  # the primary example
for vis in (800, 1200, 2500):
    for (lights, period), f in FACTOR.items():
        add({"visibility": f"{vis} m", "lights": lights, "period": period}, vis * f, f)
add({"visibility": "1500 m", "lights": "hi", "period": "night", "purpose": "planning"}, 1500.0, 1.0)
add({"visibility": "1500 m", "lights": "other", "period": "day", "purpose": "approach"}, 1500.0, 1.0)
add({"visibility": "0.75 mi", "lights": "hi", "period": "day"}, 0.75 * MI * 1.5, 1.5)
add({"visibility": "2 km", "lights": "other", "period": "night"}, 3000.0, 1.5)
bad({"visibility": "1500 m", "lights": "none", "period": "night"}, "/lights")
bad({"visibility": "1500 m", "lights": "hi", "period": "day", "purpose": "takeoff"}, "/purpose")
bad({"visibility": "1500 m", "lights": "hi", "period": "night", "purpose": "lvo-continuation"}, "/purpose")
for v in vectors:
    print(json.dumps(v, ensure_ascii=False, separators=(",", ":")))
