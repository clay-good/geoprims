#!/usr/bin/env python3
"""Golden vectors for humidity and moist air, worked by hand: e_s = 610.94
exp(17.625 T / (T + 243.04)) Pa (Alduchov and Eskridge 1996), e from the
dew point or RH, mixing ratio 622 e / (p - e) g/kg, Tv = T / (1 - 0.378 e / p),
densities p / (287.05287 T)."""
import json
import math
import sys
from pathlib import Path

SRC = "Moist-air relations worked in Python (tools/vectors/gen_humidity.py)"
SPEC = "add-aviation-suite humid-density scenario"
R = 287.05287


def es(t):
    return 610.94 * math.exp(17.625 * t / (t + 243.04))


def td_of(e):
    g = math.log(e / 610.94)
    return 243.04 * g / (17.625 - g)


def vec(i, inp, exp, src=SRC, tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": tol, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": t}


def cases():
    out = []
    for t, td, rh, p in [(30, 24, None, 1000), (15, 10, None, None), (35, None, 40, 850), (0, -5, None, 1013.25), (-20, None, 80, 700), (45, 30, None, 950)]:
        e = es(td) if td is not None else rh / 100 * es(t)
        pp = (p if p is not None else 1013.25) * 100
        tk = t + 273.15
        tv = tk / (1 - 0.378 * e / pp)
        inp = {"temperature": f"{t} degC"}
        if td is not None:
            inp["dew_point"] = f"{td} degC"
        else:
            inp["relative_humidity"] = rh
        if p is not None:
            inp["pressure"] = f"{p} hPa"
        out.append(vec(len(out) + 1, inp, {
            "result.moist_density.value": pp / (R * tv), "result.dry_density.value": pp / (R * tk),
            "result.dew_point.value": td_of(e), "result.relative_humidity": 100 * e / es(t),
            "result.vapor_pressure.value": e / 100, "result.saturation_vapor_pressure.value": es(t) / 100,
            "result.mixing_ratio": 622 * e / (pp - e), "result.virtual_temperature.value": tv - 273.15}))
    out[0]["source"] = SPEC + " (30 °C, 1,000 hPa, dew point 24 °C: moist density below dry)"
    for inp in [{"temperature": "20 degC", "dew_point": "25 degC"}, {"temperature": "20 degC"}, {"temperature": "20 degC", "dew_point": "10 degC", "relative_humidity": 50}, {"temperature": "60 degC", "relative_humidity": 50}]:
        out.append(vec(len(out) + 1, inp, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "aviation.atmosphere.humidity.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in cases()))


if __name__ == "__main__":
    main()
