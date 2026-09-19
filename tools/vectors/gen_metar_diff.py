#!/usr/bin/env python3
"""METAR differential fixture from python-metar, a separately written decoder:

  pip install metar==1.11.0
  curl -s "https://aviationweather.gov/api/data/metar?bbox=24,-125,50,-66&format=raw" > us.txt
  curl -s "https://aviationweather.gov/api/data/metar?bbox=36,-10,60,25&format=raw" > eu.txt
  python tools/vectors/gen_metar_diff.py us.txt eu.txt

Writes core/crates/gp-aviation/tests/data/metar_diff.jsonl: each live report
(Aviation Weather Center data API) with python-metar's wind, visibility,
temperature, dew point, altimeter, sea-level pressure, and cloud layers.
Reports whose body python-metar cannot parse are left out; the file records how many.
"""
import json
import sys
from importlib.metadata import version
from pathlib import Path

from metar.Metar import Metar


def decode(raw):
    m = Metar(raw, strict=False)
    if m._unparsed_groups:
        return None
    out = {"report": raw}
    if m.wind_speed is not None:
        out["wind_speed"] = m.wind_speed.value("KT")
        if m.wind_dir is not None:
            out["wind_direction"] = m.wind_dir.value()
        if m.wind_gust is not None:
            out["wind_gust"] = m.wind_gust.value("KT")
    if m.vis is not None:
        out["visibility_m"] = m.vis.value("M")
    if m.temp is not None:
        out["temperature"] = m.temp.value("C")
    if m.dewpt is not None:
        out["dew_point"] = m.dewpt.value("C")
    if m.press is not None:
        out["altimeter_hpa"] = m.press.value("MB")
    if m.press_sea_level is not None:
        out["sea_level_pressure"] = m.press_sea_level.value("MB")
    out["clouds"] = [[cover, h.value("FT") if h is not None else None] for cover, h, _ in m.sky]
    return out


def main():
    rows, skipped = [], 0
    for path in sys.argv[1:]:
        for raw in Path(path).read_text().splitlines():
            raw = raw.strip()
            if not raw:
                continue
            try:
                row = decode(raw)
            except Exception:
                row = None
            if row is None:
                skipped += 1
            else:
                rows.append(row)
    head = {"note": f"python-metar {version('metar')} on live Aviation Weather Center reports; {len(rows)} decoded, {skipped} left out (body groups python-metar does not parse)"}
    out = Path("core/crates/gp-aviation/tests/data/metar_diff.jsonl")
    out.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in [head] + rows))
    print(head["note"])


if __name__ == "__main__":
    main()
