#!/usr/bin/env python3
"""TAF differential fixture from pytaf, a separately written TAF parser:

  pip install pytaf==1.2.1
  curl -s "https://aviationweather.gov/api/data/taf?bbox=24,-125,50,-66&format=raw" > taf_us.txt
  curl -s "https://aviationweather.gov/api/data/taf?bbox=36,-10,60,25&format=raw" > taf_eu.txt
  python tools/vectors/gen_taf_diff.py taf_us.txt taf_eu.txt

Writes core/crates/gp-aviation/tests/data/taf_diff.jsonl: each live TAF
(Aviation Weather Center data API, joined onto one line) with pytaf's
change groups: type, probability, start and end day and hour, wind,
visibility, and cloud layers. pytaf is given the forecast without its RMK
section (it reads remarks as another group), and empty groups it emits are
dropped. TAFs pytaf cannot parse are left out; the header records how many.
"""
import json
import sys
from pathlib import Path

from pytaf import TAF


def reports(text):
    out, cur = [], []
    for line in text.splitlines():
        s = line.strip()
        if not s:
            continue
        if s.startswith("TAF ") and cur:
            out.append(" ".join(cur))
            cur = []
        cur.append(s)
    if cur:
        out.append(" ".join(cur))
    return out


def group(g):
    h, w, v = g["header"], g["wind"], g["visibility"]
    out = {"type": h.get("type") or "BASE", "probability": h.get("probability"),
           "from": [h.get("from_date"), h.get("from_hours")] if h.get("from_date") else None,
           "till": [h.get("till_date"), h.get("till_hours")] if h.get("till_date") else None}
    if w:
        out["wind"] = [w["direction"], w["speed"], w["gust"], w["unit"]]
    if v:
        out["visibility"] = [v.get("more"), v.get("range"), v.get("unit")]
    out["clouds"] = [[c["layer"], c.get("ceiling")] for c in g["clouds"]]
    if g.get("vertical_visibility"):
        out["clouds"].append(["VV", g["vertical_visibility"]])
    return out


def main():
    rows, skipped = [], 0
    for path in sys.argv[1:]:
        for raw in reports(Path(path).read_text()):
            try:
                # pytaf reads a trailing RMK section as another forecast group, so give it the forecast only.
                t = TAF(raw.split(" RMK ")[0])
                # pytaf can emit an empty group (no header, wind, visibility, weather, or clouds); drop it.
                gs = [g for g in t.get_groups() if g["header"] or g["wind"] or g["visibility"] or g["clouds"] or g["weather"]]
                rows.append({"report": raw, "groups": [group(g) for g in gs]})
            except Exception:
                skipped += 1
    head = {"note": f"pytaf 1.2.1 on live Aviation Weather Center TAFs; {len(rows)} parsed, {skipped} left out as unparsed by pytaf"}
    Path("core/crates/gp-aviation/tests/data/taf_diff.jsonl").write_text(
        "".join(json.dumps(r, ensure_ascii=False) + "\n" for r in [head] + rows))
    print(head["note"])


if __name__ == "__main__":
    main()
