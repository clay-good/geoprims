#!/usr/bin/env python3
"""Replacement vectors for indexing.s2.covering 2.0.0, whose rectangles and
caps now go through S2's own RegionCoverer (core/crates/gp-indexing/src/
s2exact.rs). The earlier vectors pinned the old refinement's own cells; each
whose expectation S2 does not reproduce is superseded by one computed here by
s2sphere's RegionCoverer on the same input (cap angle = radius / 6,371,008.8
m, as the tool converts it). The old lines keep their place and gain
supersededBy and a reason; new lines are appended."""
import json
import re
from pathlib import Path

import s2sphere as s2

PATH = Path("core/vectors/indexing.s2.covering.jsonl")
TAG = "gen_s2_cover_supersede.py"
EARTH_M = 6_371_008.8
REASON = "Coverings of rectangles and caps now come from S2's own RegionCoverer (2.0.0); this vector pinned the earlier refinement's cells."


def num(v, unit):
    if isinstance(v, (int, float)):
        return float(v)
    m = re.fullmatch(r"\s*(-?[\d.]+)\s*(\w*)\s*", v)
    x, u = float(m.group(1)), m.group(2) or unit
    return x * {"deg": 1, "m": 1, "km": 1000}[u]


def expected(inp):
    cov = s2.RegionCoverer()
    cov.min_level = int(inp.get("min_level", 0))
    cov.max_level = int(inp.get("max_level", 16))
    cov.max_cells = int(inp.get("max_cells", 8))
    if "radius" in inp:
        axis = s2.LatLng.from_degrees(num(inp["lat"], "deg"), num(inp["lon"], "deg")).to_point()
        region = s2.Cap.from_axis_angle(axis, s2.Angle.from_radians(num(inp["radius"], "m") / EARTH_M))
    else:
        region = s2.LatLngRect(s2.LatLng.from_degrees(num(inp["south"], "deg"), num(inp["west"], "deg")),
                               s2.LatLng.from_degrees(num(inp["north"], "deg"), num(inp["east"], "deg")))
    cells = cov.get_covering(region)
    levels = [c.level() for c in cells]
    k = len(cells) - 1
    return {"result.count": len(cells), "result.coarsest_level": min(levels), "result.finest_level": max(levels),
            "result.cells.0.cell": cells[0].to_token(), f"result.cells.{k}.cell": cells[k].to_token(), "ok": True}


def main():
    raw = PATH.read_text().splitlines()
    lines = [(l, json.loads(l)) for l in raw if TAG not in l]
    n0, new, out = len(lines), [], []
    for text, v in lines:
        if v.get("supersededBy") or "polygon" in v["input"] or v["expect"].get("ok") is False:
            out.append(text)
            continue
        want = expected(v["input"])
        if all(want.get(k) == x for k, x in v["expect"].items()):
            out.append(text)
            continue
        nid = f"v{n0 + len(new) + 1:03d}"
        # The line keeps its bytes; only the two fields are added at its end.
        out.append(text[:-1] + "," + json.dumps({"supersededBy": nid, "reason": REASON}, ensure_ascii=False, separators=(",", ":"))[1:])
        new.append({"id": nid, "input": v["input"], "expect": want,
                    "source": f"s2sphere 0.2.5 RegionCoverer on the same input ({TAG})",
                    "sourceVersion": "s2sphere 0.2.5",
                    "tolerance": {k: {"abs": 0} for k in want if k in ("result.count", "result.coarsest_level", "result.finest_level")}})
    PATH.write_text("".join(l + "\n" for l in out) + "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in new))
    print("superseded", len(new))


if __name__ == "__main__":
    main()
