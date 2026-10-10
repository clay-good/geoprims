#!/usr/bin/env python3
"""Golden vectors for aviation.atmosphere.humidity from MetPy, Unidata's
meteorology library, written independently of geoprims.

MetPy takes saturation vapor pressure from Bolton (1980); the tool uses the
Magnus form of Alduchov and Eskridge (1996). The two formulas differ by up to
0.31% between -40 and 45 degC, so vapor pressures, relative humidity, and
mixing ratio are held to 0.4%, the dew point to 0.05 degC, and the virtual
temperature to 0.02 degC. Air density depends little on which is used and is
held to 5 parts in 100,000.

Replaces only its own rows in the tool's vector file.

Usage: python3 tools/vectors/gen_humidity_metpy.py   (with metpy installed)
"""
import json
from pathlib import Path

import metpy
import metpy.calc as mc
from metpy.units import units as u

SRC = f"MetPy {metpy.__version__} (metpy.calc saturation_vapor_pressure, mixing_ratio, virtual_temperature, density, dewpoint_from_relative_humidity) (tools/vectors/gen_humidity_metpy.py)"
OUT = Path(__file__).resolve().parents[2] / "core/vectors/aviation.atmosphere.humidity.jsonl"
VAPOR, DENSITY = {"rel": 4e-3}, {"rel": 5e-5}

rows = [r for r in map(json.loads, OUT.read_text().splitlines()) if "MetPy" not in r["source"]]
# temperature degC, dew point degC or None, relative humidity % or None, pressure hPa
CASES = [(30, 24, None, 1000), (15, 10, None, 1013.25), (-20, -25, None, 700), (45, 30, None, 1010), (0, -5, None, 900), (22, 21.5, None, 950),
         (-40, -45, None, 500), (10, -10, None, 800), (40, 5, None, 1013.25), (5, 5, None, 1020), (35, None, 40, 850), (25, None, 90, 1005),
         (-10, None, 55, 750), (18, None, 20, 880), (38, None, 65, 1013.25), (2, None, 100, 990)]
for t, td, rh, p in CASES:
    tq, pq = t * u.degC, p * u.hPa
    inp = {"temperature": f"{t} degC", "pressure": f"{p} hPa"}
    if td is None:
        inp["relative_humidity"] = rh
        tdq = mc.dewpoint_from_relative_humidity(tq, rh * u.percent)
    else:
        inp["dew_point"] = f"{td} degC"
        tdq = td * u.degC
    e = mc.saturation_vapor_pressure(tdq)
    w = mc.mixing_ratio(e, pq)
    exp = {"ok": True,
           "result.moist_density.value": float(mc.density(pq, tq, w).to("kg/m^3").m),
           "result.dry_density.value": float(mc.density(pq, tq, 0 * u("g/kg")).to("kg/m^3").m),
           "result.dew_point.value": float(tdq.to("degC").m),
           "result.relative_humidity": float(mc.relative_humidity_from_dewpoint(tq, tdq).to("percent").m),
           "result.vapor_pressure.value": float(e.to("hPa").m),
           "result.saturation_vapor_pressure.value": float(mc.saturation_vapor_pressure(tq).to("hPa").m),
           "result.mixing_ratio": float(w.to("g/kg").m),
           "result.virtual_temperature.value": float(mc.virtual_temperature(tq, w).to("degC").m)}
    tol = {"result.moist_density.value": DENSITY, "result.dry_density.value": DENSITY, "result.dew_point.value": {"abs": 0.05},
           "result.relative_humidity": VAPOR, "result.vapor_pressure.value": VAPOR, "result.saturation_vapor_pressure.value": VAPOR,
           "result.mixing_ratio": VAPOR, "result.virtual_temperature.value": {"abs": 0.02}}
    rows.append({"id": f"v{len(rows) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": f"MetPy {metpy.__version__}", "tolerance": tol})
OUT.write_text("".join(json.dumps(r, ensure_ascii=False, separators=(",", ":")) + "\n" for r in rows))
print(f"{OUT.name}: {len(rows)} vectors")
