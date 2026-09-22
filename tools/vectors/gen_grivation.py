#!/usr/bin/env python3
"""Golden vectors for geodesy.magnetic.grivation: grid variation G = D − γ,
with D from the NCEI WMM2025 test values (as published, to 0.01°, taken from
the declination tool's vectors) and γ from GeographicLib GeoConvert -c (UTM
or UPS as chosen; -z 0 forces UPS)."""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SRC = "NCEI WMM2025 test values (declination) and GeographicLib GeoConvert -c (convergence)"
VER = "WMM2025 (2024-11-13); GeographicLib 2.x"


def convergence(lat, lon, ups):
    cmd = ["GeoConvert", "-c", "-p", "9"] + (["-z", "0"] if ups else [])
    return float(subprocess.run(cmd, input=f"{lat} {lon}\n", capture_output=True, text=True, check=True).stdout.split()[0])


def main():
    decl = {}
    for line in (ROOT / "core/vectors/geodesy.magnetic.declination.jsonl").read_text().splitlines():
        v = json.loads(line)
        if v["source"].startswith("NCEI WMM2025 test values") and "result.declination.value" in v["expect"]:
            decl[v["id"]] = (v["input"], v["expect"]["result.declination.value"])
    out = []
    for vid, grid in [("v001", "auto"), ("v001", "ups"), ("v024", "auto"), ("v027", "auto"), ("v025", "utm"), ("v006", "auto"), ("v005", "auto")]:
        inp, d = decl[vid]
        lat, lon = inp["lat"], inp["lon"]
        ups = grid == "ups" or (grid == "auto" and not (-80 <= lat <= 84))
        g = convergence(lat, lon, ups)
        gv = (d - g + 180) % 360 - 180
        i2 = dict(inp, grid=grid)
        exp = {"result.grivation.value": gv, "result.declination.value": d, "result.convergence.value": g, "ok": True}
        tol = {"result.grivation.value": {"abs": 0.006}, "result.declination.value": {"abs": 0.006}, "result.convergence.value": {"abs": 1e-7}}
        out.append({"id": f"v{len(out) + 1:03d}", "input": i2, "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
    # UTM does not reach 89° N, and UPS does not come down to 72° S.
    for vid, grid in [("v001", "utm"), ("v024", "ups")]:
        out.append({"id": f"v{len(out) + 1:03d}", "input": dict(decl[vid][0], grid=grid), "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
                    "source": SRC, "sourceVersion": VER, "tolerance": {}})
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "geodesy.magnetic.grivation.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
