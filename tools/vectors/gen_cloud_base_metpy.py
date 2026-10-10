#!/usr/bin/env python3
"""Golden vectors for the lifting condensation level in
aviation.atmosphere.cloud-base from MetPy, Unidata's meteorology library,
written independently of geoprims.

MetPy finds the LCL by iterating on the parcel's saturation; the tool uses
Bolton's (1980) closed fit for its temperature. The height above the surface
is the dry-adiabatic climb to that temperature, (T - T_LCL) / (g / c_p), with
MetPy's own g / c_p (9.761 K per km against the tool's 9.77). The two agree
within 0.08% from -20 to 45 degC and are held to 0.1%. MetPy's LCL temperature
does not depend on the surface pressure, which the tool does not ask for; this
is checked at 1013.25, 850, and 700 hPa.

The 400 ft per degree rule and the freezing level are plain arithmetic and are
not in these vectors.

Replaces only its own rows in the tool's vector file.

Usage: python3 tools/vectors/gen_cloud_base_metpy.py   (with metpy installed)
"""
import json
from pathlib import Path

import metpy
import metpy.calc as mc
import metpy.constants as const
from metpy.units import units as u

SRC = f"MetPy {metpy.__version__} (metpy.calc.lcl and metpy.constants.dry_adiabatic_lapse_rate) (tools/vectors/gen_cloud_base_metpy.py)"
OUT = Path(__file__).resolve().parents[2] / "core/vectors/aviation.atmosphere.cloud-base.jsonl"
TOL = {"rel": 1e-3}

# Lines from other generators are kept byte for byte.
kept = [line for line in OUT.read_text().splitlines() if "gen_cloud_base_metpy.py" not in json.loads(line)["source"]]
rows = []
# temperature degC, dew point degC, field elevation ft or None
for t, td, elev in [(25, 15, None), (30, 10, 5430), (15, 14, 0), (8, 2, 2000), (35, 5, 1000), (-5, -9, 6000), (40, -10, 2180), (20, 19.5, None),
                    (12, 0, 4500), (-20, -28, 440), (28, 22, 13), (5, -15, 7200), (33, 18, 620), (18, 6, None), (0, -1, 1500), (45, 0, -200)]:
    heights = []
    for p in (1013.25, 850, 700):
        _, t_lcl = mc.lcl(p * u.hPa, t * u.degC, td * u.degC)
        heights.append((((t * u.degC).to("K") - t_lcl.to("K")) / const.dry_adiabatic_lapse_rate).to("ft").m)
    assert max(heights) - min(heights) < 1e-6 * max(heights), heights
    inp = {"temperature": f"{t} degC", "dew_point": f"{td} degC"}
    exp = {"ok": True, "result.cloud_base_lcl.value": float(heights[0])}
    tol = {"result.cloud_base_lcl.value": TOL}
    if elev is not None:
        inp["elevation"] = f"{elev} ft"
        exp["result.cloud_base_msl.value"] = float(heights[0]) + elev
        tol["result.cloud_base_msl.value"] = {"abs": 1e-3 * min(float(heights[0]), float(heights[0]) + elev)}
    rows.append({"id": f"v{len(kept) + len(rows) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": f"MetPy {metpy.__version__}", "tolerance": tol})
OUT.write_text("".join(line + "\n" for line in kept + [json.dumps(r, ensure_ascii=False, separators=(",", ":")) for r in rows]))
print(f"{OUT.name}: {len(kept) + len(rows)} vectors")
