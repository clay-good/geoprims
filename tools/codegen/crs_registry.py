#!/usr/bin/env python3
"""Generates core/crates/gp-geo/src/crs_registry.rs: the compact CRS index
the CRS search tool reads, from the EPSG dataset through the projinfo command
of PROJ 9.9.0 (EPSG v13.102), the same database and version the crs-registry
asset is built from (tools/data/crs-registry.mjs). Names, units, and areas of
use are EPSG's own; nothing is typed by hand.

The index holds what the converters here can produce:
- four geographic CRSs (WGS 84, NAD83, NAD83(2011), NAD27) and Web Mercator;
- every UTM zone on WGS 84, NAD83, and NAD83(2011);
- every SPCS83 zone on NAD83 and NAD83(2011), in meters and in each foot EPSG
  defines for it (US survey or international), tied to the zone's NGS code.

Deprecated EPSG entries are left out. Needs projinfo and sqlite3 on the PATH.
Rerunning must reproduce the file.
"""
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "core/crates/gp-geo/src/crs_registry.rs"
ZONES = ROOT / "core/crates/gp-geo/src/spcs83_zones.rs"
UNITS = {"degree": "deg", "metre": "m", "US survey foot": "ftUS", "foot": "ft"}
GEOGRAPHIC = [4326, 4269, 6318, 4267]
EPSG_VERSION, EPSG_DATE = "v13.102", "2026-08-27"


def lit(x):
    r = repr(float(x))
    return r if ("." in r or "e" in r) else r + ".0"


def esc(s):
    return s.replace("\\", "\\\\").replace('"', '\\"')


def run(*args):
    return subprocess.run(args, check=True, capture_output=True, text=True).stdout


def metadata():
    db = Path(run("projinfo", "--searchpaths").strip().splitlines()[-1]) / "proj.db"
    rows = dict(line.split("=", 1) for line in run("sqlite3", str(db), "select key||'='||value from metadata;").strip().splitlines())
    assert (rows["EPSG.VERSION"], rows["EPSG.DATE"]) == (EPSG_VERSION, EPSG_DATE), rows
    return rows["PROJ.VERSION"]


def definition(code):
    return json.loads(run("projinfo", "-q", f"EPSG:{code}", "-o", "PROJJSON", "--single-line"))


def main():
    proj = metadata()
    infos = {}
    for line in run("projinfo", "--list-crs").splitlines():
        m = re.match(r'^EPSG:(\d+) "(.*)"( \[deprecated\])?$', line)
        if m and not m.group(3):
            infos[m.group(2)] = int(m.group(1))
    rows = []

    def add(code, kind):
        d = definition(code)
        unit = d["coordinate_system"]["axis"][0]["unit"]
        unit = UNITS[unit if isinstance(unit, str) else unit["name"]]
        b = d["bbox"]
        rows.append((code, f'    Crs {{ code: {code}, name: "{esc(d["name"])}", kind: {kind}, unit: "{unit}", '
                           f'bbox: [{lit(b["west_longitude"])}, {lit(b["south_latitude"])}, {lit(b["east_longitude"])}, {lit(b["north_latitude"])}] }},'))

    for code in GEOGRAPHIC:
        add(code, "CrsKind::Geographic")
    add(3857, "CrsKind::WebMercator")

    utm = re.compile(r"^(WGS 84|NAD83|NAD83\(2011\)) / UTM zone (\d+)([NS])$")
    for name, code in infos.items():
        m = utm.match(name)
        if m:
            add(code, f"CrsKind::Utm {{ zone: {int(m.group(2))}, north: {str(m.group(3) == 'N').lower()} }}")

    # SPCS83: each zone's NAD83 metre CRS is in the zone table with its NGS
    # code; its twins differ only in datum label and unit suffix.
    fips_of = {int(e): f for f, e in re.findall(r'fips: "(\d+)", epsg: (\d+)', ZONES.read_text())}
    assert len(fips_of) == 124, len(fips_of)
    # Each twin keeps its own false origin: EPSG defines the feet twins with
    # an origin rounded in feet (3,000,000 ftUS for Colorado Central), which
    # is not the metric zone's 914,401.8289 m converted. Every other
    # parameter must equal the metric zone's.
    to_m = {"metre": 1.0, "US survey foot": 1200 / 3937, "foot": 0.3048}

    def params(code):
        out = []
        for x in definition(code)["conversion"]["parameters"]:
            u = x.get("unit")
            u = u if isinstance(u, str) else u["name"]
            out.append((x["name"], x["value"] * to_m.get(u, 1.0), "easting" in x["name"].lower() or "northing" in x["name"].lower()))
        return out

    for epsg, fips in sorted(fips_of.items()):
        base = definition(epsg)["name"].removeprefix("NAD83 / ")
        metric = params(epsg)
        found = 0
        for datum in ("NAD83", "NAD83(2011)"):
            for suffix in ("", " (ftUS)", " (ft)"):
                code = infos.get(f"{datum} / {base}{suffix}")
                if code:
                    own = params(code)
                    assert [(n, o) for n, _, o in own] == [(n, o) for n, _, o in metric], code
                    for (n, v, origin), (_, w, _) in zip(own, metric):
                        if not origin:
                            assert abs(v - w) <= 1e-9 * max(1.0, abs(w)), (code, n, v, w)
                    fe, fn_ = [v for n, v, origin in own if origin]
                    add(code, f'CrsKind::Spcs83 {{ fips: "{fips}", fe: {lit(fe)}, fn_: {lit(fn_)} }}')
                    found += 1
        assert found >= 1, (epsg, base, found)

    rows.sort()
    codes = [c for c, _ in rows]
    assert len(codes) == len(set(codes)), "duplicate code"
    body = "\n".join(r for _, r in rows)
    OUT.write_text(
        f"// Generated by tools/codegen/crs_registry.py from EPSG {EPSG_VERSION} in PROJ {proj}. Do not edit.\n"
        "use crate::crs::{Crs, CrsKind};\n\n"
        "#[rustfmt::skip]\n"
        f"pub const REGISTRY: &[Crs] = &[\n{body}\n];\n"
    )
    print(f"wrote {len(rows)} CRSs to {OUT}")


if __name__ == "__main__":
    main()
