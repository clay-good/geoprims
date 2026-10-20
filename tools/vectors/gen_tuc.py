#!/usr/bin/env python3
"""Golden vectors for aviation.physiology.time-of-useful-consciousness
(add-aviation-practical-additions, aviation/physiology-references).

Rows as printed in AC 61-107B Change 1 Figure 2-3 (9/9/15, page 24) and
PHAK-25C Figure 17-1, both read at faa.gov on 2026-10-20. The tool quotes
rows and never interpolates: between rows it reports the two rows around
the altitude. Writes nothing: new vectors are appended by hand so
published files stay frozen.
"""
import json

AC = [(18000, "20–30 min", "10–15 min"), (22000, "10 min", "5–6 min"), (25000, "3–5 min", "1.5–2.5 min"),
      (28000, "2.5–3 min", "1–1.5 min"), (30000, "1–2 min", "30 s–1 min"), (35000, "30 s–1 min", "15–30 s"),
      (40000, "15–20 s", "nominal"), (43000, "9–12 s", "nominal"), (50000, "9–12 s", "nominal")]
PHAK = [(20000, "30 min or more"), (22000, "5–10 min"), (25000, "3–5 min"), (28000, "2.5–3 min"),
        (30000, "1–2 min"), (35000, "30–60 s"), (40000, "15–20 s"), (45000, "9–15 s")]
SRC_AC = "AC 61-107B Change 1 Figure 2-3, as printed"
SRC_BOTH = "AC 61-107B Change 1 Figure 2-3 and PHAK-25C Figure 17-1, as printed"
VER = "Change 1, 9/9/15"


def look(rows, h, col):
    alts = [r[0] for r in rows]
    if h in alts:
        return rows[alts.index(h)][col]
    hi = next(i for i, a in enumerate(alts) if a > h)
    return f"{rows[hi - 1][col]} at {rows[hi - 1][0]:,} ft; {rows[hi][col]} at {rows[hi][0]:,} ft"


vectors = []


def add(h, src):
    exp = {"ok": True, "result.tuc": look(AC, h, 1), "result.tuc_rapid": look(AC, h, 2)}
    if PHAK[0][0] <= h <= PHAK[-1][0]:
        exp["result.tuc_phak"] = look(PHAK, h, 1)
    vectors.append({"id": f"v{len(vectors) + 1:03d}", "input": {"altitude": f"{h} ft"}, "expect": exp,
                    "source": src, "sourceVersion": VER, "tolerance": {}})


add(25000, SRC_BOTH)  # the primary example
for h, _, _ in AC:
    if h != 25000:
        add(h, SRC_BOTH if PHAK[0][0] <= h <= PHAK[-1][0] else SRC_AC)
for h in [45000]:
    add(h, SRC_BOTH)
# Between rows: both rows, not interpolated.
for h in [19000, 21000, 23000, 26500, 29000, 32000, 37000, 41500, 44000]:
    add(h, SRC_BOTH if PHAK[0][0] <= h <= PHAK[-1][0] else SRC_AC)
for h in [17999, 50001]:
    vectors.append({"id": f"v{len(vectors) + 1:03d}", "input": {"altitude": f"{h} ft"},
                    "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN", "error.field": "/altitude"},
                    "source": SRC_AC, "sourceVersion": VER, "tolerance": {}})
for v in vectors:
    print(json.dumps(v, ensure_ascii=False, separators=(",", ":")))
