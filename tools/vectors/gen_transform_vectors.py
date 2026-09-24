#!/usr/bin/env python3
"""Vectors for geodesy.datum.transform (add-geodesy-suite task 3.3) from the
independent references already pinned for the single-transformation tools:
NGS HTDP 3.6.0 outputs where NAD 83 is one of the frames (geodesy.datum.nad83)
and PROJ's IERS parameter file (geodesy.datum.itrf). For each of these the frame graph takes the same path
as the dedicated tool, so the reference values carry over unchanged; the
source names the vector they came from.

Appends to core/vectors/geodesy.datum.transform.jsonl (existing lines left
byte for byte)."""
import json
from pathlib import Path

PATH = Path("core/vectors/geodesy.datum.transform.jsonl")
TAG = "gen_transform_vectors.py"
FROM = [("geodesy.datum.nad83", "NGS HTDP"), ("geodesy.datum.itrf", "PROJ +proj=helmert")]
KEEP = ("result.lat.value", "result.lon.value", "result.height.value", "ok")


def main():
    lines = PATH.read_text().splitlines() if PATH.exists() else []
    kept = [l for l in lines if TAG not in l]
    n0, new = len(kept), []
    for tool, prefix in FROM:
        for l in Path(f"core/vectors/{tool}.jsonl").read_text().splitlines():
            v = json.loads(l)
            if not v["source"].startswith(prefix) or v.get("supersededBy") or "lat" not in v["input"]:
                continue
            # HTDP's own sets between ITRF and WGS 84 frames are not the path
            # the graph takes there (IERS and NGA's alignment, which differ by
            # up to 2 cm); only its NAD 83 steps are shared.
            if tool.endswith("nad83") and "NAD83" not in v["input"]["from"] + v["input"]["to"]:
                continue
            inp = {k: v["input"][k] for k in ("lat", "lon", "height", "from", "to", "epoch") if k in v["input"]}
            exp = {k: val for k, val in v["expect"].items() if k in KEEP}
            tol = {k: t for k, t in v["tolerance"].items() if k in exp}
            if not any(k.startswith("result.") for k in exp):
                continue
            new.append({"id": f"v{n0 + len(new) + 1:03d}", "input": inp, "expect": exp,
                        "source": f"{v['source']}, as pinned in {tool} {v['id']} ({TAG})",
                        "sourceVersion": v["sourceVersion"], "tolerance": tol})
    PATH.write_text("".join(l + "\n" for l in kept) + "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in new))
    print(n0, "->", n0 + len(new))


if __name__ == "__main__":
    main()
