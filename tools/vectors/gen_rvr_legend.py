#!/usr/bin/env python3
"""Golden vectors for the chart legend option of aviation.ifr.rvr-visibility
(add-aviation-practical-additions task 8): the Terminal Procedures
Publication legend's Comparable Values of RVR and Visibility, as printed in
the d-TPP cycle 2611 front matter (29 OCT 2026 to 26 NOV 2026), with its
rule to use the next higher listed RVR between rows. Rows the regulation
(14 CFR 91.175(h)(2)) does not list carry RVR_NOT_IN_REGULATION.

    gen_rvr_legend.py FIRST_ID    # print the vectors, numbered from FIRST_ID

The earlier vectors of this tool cover the regulation mode. Writes nothing:
new vectors are appended by hand so published files stay frozen.
"""
import json
import sys

LEGEND = [(1200, "1/4", .25), (1600, "1/4", .25), (1800, "1/2", .5), (2000, "1/2", .5), (2200, "1/2", .5),
          (2400, "1/2", .5), (2600, "1/2", .5), (3000, "5/8", .625), (3200, "5/8", .625), (3500, "5/8", .625),
          (4000, "3/4", .75), (4500, "7/8", .875), (5000, "1", 1.0), (5500, "1", 1.0), (6000, "1 1/4", 1.25)]
REGULATION = {1600, 2400, 3200, 4000, 4500, 5000, 6000}
SRC = "Terminal Procedures Publication legend, d-TPP cycle 2611, as printed"
VER = "d-TPP cycle 2611"

first = int(sys.argv[1])
vectors = []


def add(rvr, row):
    used, vis, sm = row
    exp = {"ok": True, "result.visibility": vis, "result.visibility_sm.value": sm, "result.rvr_used.value": float(used)}
    if used not in REGULATION:
        exp["meta.warnings.*.code"] = "RVR_NOT_IN_REGULATION"
    vectors.append({"id": f"v{first + len(vectors):03d}", "input": {"rvr": f"{rvr} ft", "table": "legend"}, "expect": exp,
                    "source": SRC, "sourceVersion": VER,
                    "tolerance": {"result.visibility_sm.value": {"abs": 1e-9}, "result.rvr_used.value": {"abs": 1e-9}}})


add(4800, LEGEND[12])  # the legend's own example: use 5000, 1 mile
for row in LEGEND:
    add(row[0], row)
add(2800, LEGEND[7])  # between 2,600 and 3,000: the next higher row
for rvr in (1100, 6100):
    vectors.append({"id": f"v{first + len(vectors):03d}", "input": {"rvr": f"{rvr} ft", "table": "legend"},
                    "expect": {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/rvr"},
                    "source": SRC, "sourceVersion": VER, "tolerance": {}})
for v in vectors:
    print(json.dumps(v, ensure_ascii=False, separators=(",", ":")))
