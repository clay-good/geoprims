#!/usr/bin/env python3
"""Build the polygon-overlay fixture from JTS's canonical XML geometries.

Usage:
    python3 tools/vectors/gen_overlay_jts.py /path/to/jts-1.20.0

The JTS checkout must be exactly the commit behind tag 1.20.0.  The source
geometries come from its four area/area overlay suites.  GEOS then runs all
four boolean operations in floating precision, independent of the expected
answers stored in the XML.  The compact, committed fixture keeps CI offline.
"""

from __future__ import annotations

import json
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

JTS_COMMIT = "6e95fe82feb986a7aa657f4ffa406d8c290af509"
SOURCE_ROOT = Path("modules/tests/src/test/resources/testxml/general")
SOURCE_FILES = (
    "TestOverlayAA.xml",
    "TestOverlayAAPrec.xml",
    "TestNGOverlayA.xml",
    "TestNGOverlayAPrec.xml",
)
OPERATIONS = ("intersection", "union", "difference", "symDifference")
OUT = Path("core/crates/gp-geometry/tests/data/overlay_jts_geos.json")


def run(*args: str, cwd: Path | None = None) -> str:
    return subprocess.run(
        args,
        cwd=cwd,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    ).stdout.strip()


def geos(a: str, operation: str = "copy", b: str | None = None) -> dict:
    args = ["geosop", "-a", a]
    if b is not None:
        args += ["-b", b]
    args += ["-f", "geojson", operation]
    return json.loads(run(*args))


def polygon_rings(geometry: dict) -> list[list[list[float]]] | None:
    if geometry.get("type") != "Polygon":
        return None
    rings = []
    for ring in geometry.get("coordinates", []):
        if not ring:
            continue
        points = ring[:-1] if len(ring) > 1 and ring[0] == ring[-1] else ring
        rings.append([[float(x), float(y)] for x, y, *_ in points])
    return rings or None


def polygon_parts(geometry: dict) -> list[list[list[list[float]]]]:
    kind = geometry.get("type")
    if kind == "Polygon":
        rings = polygon_rings(geometry)
        return [rings] if rings else []
    if kind == "MultiPolygon":
        out = []
        for coordinates in geometry.get("coordinates", []):
            rings = polygon_rings({"type": "Polygon", "coordinates": coordinates})
            if rings:
                out.append(rings)
        return out
    if kind == "GeometryCollection":
        out = []
        for member in geometry.get("geometries", []):
            out.extend(polygon_parts(member))
        return out
    return []


def text(element: ET.Element) -> str:
    return "".join(element.itertext()).strip()


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: gen_overlay_jts.py /path/to/jts-1.20.0")
    checkout = Path(sys.argv[1]).resolve()
    commit = run("git", "rev-parse", "HEAD", cwd=checkout)
    if commit != JTS_COMMIT:
        raise SystemExit(f"JTS checkout is {commit}; expected {JTS_COMMIT} (tag 1.20.0)")

    help_text = run("geosop", "-h")
    geos_version = help_text.splitlines()[0].removeprefix("geosop - ")
    cases = []
    for filename in SOURCE_FILES:
        source = checkout / SOURCE_ROOT / filename
        root = ET.parse(source).getroot()
        for index, case in enumerate(root.findall("case"), 1):
            a_element, b_element = case.find("a"), case.find("b")
            if a_element is None or b_element is None:
                continue
            a_wkt, b_wkt = text(a_element), text(b_element)
            try:
                a, b = polygon_rings(geos(a_wkt)), polygon_rings(geos(b_wkt))
            except (subprocess.CalledProcessError, json.JSONDecodeError):
                continue
            if not a or not b:
                continue
            results = {
                operation: polygon_parts(geos(a_wkt, operation, b_wkt))
                for operation in OPERATIONS
            }
            description = (case.findtext("desc") or "").strip()
            cases.append(
                {
                    "source": f"{filename} case {index}",
                    "description": " ".join(description.split()),
                    "a": a,
                    "b": b,
                    "results": results,
                }
            )

    fixture = {
        "source": {
            "project": "JTS Topology Suite",
            "version": "1.20.0",
            "commit": JTS_COMMIT,
            "files": list(SOURCE_FILES),
            "url": "https://github.com/locationtech/jts/tree/1.20.0/modules/tests/src/test/resources/testxml/general",
            "license": "EPL-2.0 OR EDL-1.0",
        },
        "reference": geos_version,
        "operations": list(OPERATIONS),
        "cases": cases,
    }
    OUT.write_text(json.dumps(fixture, separators=(",", ":")) + "\n")
    print(f"{len(cases)} JTS polygon pairs x {len(OPERATIONS)} operations; {geos_version}")


if __name__ == "__main__":
    main()
