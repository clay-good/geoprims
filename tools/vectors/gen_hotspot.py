#!/usr/bin/env python3
"""Golden vectors for time.sun.hotspot: the sun from the NOAA solar equations
(transcribed independently in tools/vectors/gen_sun.py, within about 0.01° of
the tool's NREL SPA), the antisolar point opposite it, and the angle between
it and the camera's look vector, in the frame when within half the diagonal
field of view."""
import importlib.util
import json
import math
import sys
from datetime import datetime
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("gen_sun", HERE / "gen_sun.py")
gen_sun = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gen_sun)

SRC, VER = "NOAA solar equations (tools/vectors/gen_sun.py) and vector geometry (tools/vectors/gen_hotspot.py)", "NOAA GML 2023"


def unit(az, el):
    a, e = math.radians(az), math.radians(el)
    return (math.sin(a) * math.cos(e), math.cos(a) * math.cos(e), math.sin(e))


def main():
    cases = [(39.74, -104.99, "2026-06-21T13:00-06:00", None, -90, None),
             (39.74, -104.99, "2026-06-21T09:00-06:00", None, -90, None),
             (51.48, -0.0, "2026-03-20T12:10Z", 0, -60, 84),
             (-33.87, 151.21, "2026-12-21T12:00+11:00", 180, -45, 70),
             (35.0, -110.0, "2026-09-22T17:30-07:00", 270, -10, 60),
             (60.0, 10.0, "2026-06-21T02:00Z", None, -90, None)]
    out = []
    for i, (lat, lon, t, heading, pitch, fov) in enumerate(cases, 1):
        el, az, _ = gen_sun.noaa(lat, lon, datetime.fromisoformat(t.replace("Z", "+00:00")))
        look = unit(heading or 0, pitch)
        anti = unit((az + 180) % 360, -el)
        angle = math.degrees(math.acos(max(-1, min(1, sum(a * b for a, b in zip(look, anti))))))
        f = fov or 84
        inp = {"lat": lat, "lon": lon, "time": t, "camera_pitch": f"{pitch} deg"}
        if heading is not None:
            inp["camera_heading"] = f"{heading} deg"
        if fov is not None:
            inp["field_of_view"] = f"{fov} deg"
        in_frame = "no, the sun is down" if el <= 0 else ("yes" if angle <= f / 2 else "no")
        exp = {"result.hotspot_angle.value": angle, "result.sun_elevation.value": el, "result.sun_azimuth.value": az, "result.in_frame": in_frame, "ok": True}
        tol = {k: {"abs": 0.02} for k, v in exp.items() if isinstance(v, float)}
        out.append({"id": f"v{i:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
        print(t, round(el, 2), round(angle, 2), in_frame)
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "time.sun.hotspot.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
