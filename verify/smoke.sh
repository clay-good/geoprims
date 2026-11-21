#!/bin/sh
set -eu

fail() {
  echo "reference smoke failed: $1" >&2
  exit 1
}

geographiclib_version=$(GeodSolve --version)
case "$geographiclib_version" in
  *2.7*) ;;
  *) fail "GeographicLib version: $geographiclib_version" ;;
esac
geodesic=$(printf '0 0 0 1\n' | GeodSolve -i)
case "$geodesic" in
  *'90.00000000 90.00000000 111319.491'*) ;;
  *) fail "GeographicLib inverse result: $geodesic" ;;
esac

proj_version=$(proj 2>&1 || true)
case "$proj_version" in
  *9.9.0*) ;;
  *) fail "PROJ version: $proj_version" ;;
esac
projected=$(printf '0 0\n' | proj +proj=merc +ellps=WGS84)
case "$projected" in
  '0.00	0.00') ;;
  *) fail "PROJ Mercator result: $projected" ;;
esac

h3_version=$(h3 --help 2>&1)
case "$h3_version" in
  *4.5.0*) ;;
  *) fail "H3 version: $h3_version" ;;
esac
h3_cell=$(latLngToCell -r 9 --lat 37.775938728915946 --lng -122.41795063018799)
[ "$h3_cell" = "8928308280fffff" ] || fail "H3 cell: $h3_cell"

s2_cell=$(s2-smoke)
[ "$s2_cell" = "43ce61" ] || fail "S2 cell: $s2_cell"

wmm_output=$(mktemp)
trap 'rm -f "$wmm_output"' EXIT HUP INT TERM
(
  cd /usr/local/share/wmm2025
  wmm2025-file f sample_coords.txt "$wmm_output"
)
first_wmm_row=$(sed -n '2p' "$wmm_output")
case "$first_wmm_row" in
  *'16d 48m'*'52531.3'*) ;;
  *) fail "WMM2025 result: $first_wmm_row" ;;
esac

htdp_output=$(printf '0\n' | htdp)
case "$htdp_output" in
  *'SOFTWARE VERSION: 3.6.0'*) ;;
  *) fail "HTDP version banner" ;;
esac

# The hydrology references: a fill and a D8 pointer through each, on one grid
# both of them read (add-raster-hydrology task 2).
#
# The checks are facts the test grid makes true by construction: a 5 by 5
# plane sloping east in 1 m steps, 104 m down to 100 m, with the middle cell
# dug from 102 m to 90 m. Filling a depression raises it to its spill point --
# the lowest neighbour it can escape over, which here is the 101 m cell to its
# east, not the 102 m the plane would have put there. (CI run 44960054301
# caught that: the first version of this script asserted 102 and both tools
# rightly said 101.) Filling must also leave no depression behind, and a D8
# pointer over a plane must send every interior cell the same way. The two tools number their directions
# differently and nothing here has measured which code each version uses, so
# the script prints both codes rather than asserting one; pin them once CI has
# run this and the numbers are known.
hydro=$(mktemp -d)
trap 'rm -rf "$hydro"' EXIT HUP INT TERM
cat > "$hydro/dem.asc" <<'ASC'
ncols 5
nrows 5
xllcorner 0
yllcorner 0
cellsize 10
NODATA_value -9999
104 103 102 101 100
104 103 102 101 100
104 103  90 101 100
104 103 102 101 100
104 103 102 101 100
ASC
gdal_translate -q -of GTiff -a_srs EPSG:32631 "$hydro/dem.asc" "$hydro/dem.tif" \
  || fail "gdal_translate on the test grid"

at() { gdallocationinfo -valonly "$1" "$2" "$3"; }
# The dug cell is column 2, row 2, and its neighbours stand at 102.
pit_before=$(at "$hydro/dem.tif" 2 2)
case "$pit_before" in
  90*) ;;
  *) fail "the test grid's pit reads $pit_before, not 90" ;;
esac

whitebox_version=$(whitebox_tools --version 2>&1)
case "$whitebox_version" in
  *2.4.0*) ;;
  *) fail "WhiteboxTools version: $whitebox_version" ;;
esac
run_quietly() {
  label=$1
  shift
  if ! "$@" > "$hydro/out.txt" 2>&1; then
    sed -n '1,20p' "$hydro/out.txt" >&2
    fail "$label"
  fi
}
run_quietly "WhiteboxTools FillDepressions" \
  whitebox_tools -r=FillDepressions --dem="$hydro/dem.tif" -o="$hydro/wbt_fill.tif"
wbt_pit=$(at "$hydro/wbt_fill.tif" 2 2)
awk -v v="$wbt_pit" 'BEGIN { exit !(v >= 101 && v < 102) }' \
  || fail "WhiteboxTools filled the pit to $wbt_pit, not to its 101 m spill point"
run_quietly "WhiteboxTools D8Pointer" \
  whitebox_tools -r=D8Pointer --dem="$hydro/wbt_fill.tif" -o="$hydro/wbt_d8.tif"
wbt_dir=$(at "$hydro/wbt_d8.tif" 2 1)
[ -n "$wbt_dir" ] || fail "WhiteboxTools D8Pointer wrote no value"
for col in 1 2 3; do
  [ "$(at "$hydro/wbt_d8.tif" "$col" 1)" = "$wbt_dir" ] \
    || fail "WhiteboxTools D8Pointer disagrees across a plane at column $col"
done

richdem_version=$(rd_depressions_flood.exe 2>&1 || true)
case "$richdem_version" in
  *2.3*) ;;
  *) fail "RichDEM version banner: $richdem_version" ;;
esac
run_quietly "RichDEM rd_depressions_flood" \
  rd_depressions_flood.exe "$hydro/dem.tif" "$hydro/rd_fill.tif" 0
rd_pit=$(at "$hydro/rd_fill.tif" 2 2)
awk -v v="$rd_pit" 'BEGIN { exit !(v >= 101 && v < 102) }' \
  || fail "RichDEM filled the pit to $rd_pit, not to its 101 m spill point"
rd_has=$(rd_depressions_has.exe "$hydro/rd_fill.tif" 2>&1)
case "$rd_has" in
  *[Nn]o*depression*|*"0"*) ;;
  *) fail "RichDEM still finds a depression after filling: $rd_has" ;;
esac
run_quietly "RichDEM rd_d8_flowdirs" \
  rd_d8_flowdirs.exe "$hydro/rd_fill.tif" "$hydro/rd_d8"
rd_dir=$(at "$hydro/rd_d8-flowdirs.tif" 2 1)
[ -n "$rd_dir" ] || fail "RichDEM rd_d8_flowdirs wrote no value"
for col in 1 2 3; do
  [ "$(at "$hydro/rd_d8-flowdirs.tif" "$col" 1)" = "$rd_dir" ] \
    || fail "RichDEM rd_d8_flowdirs disagrees across a plane at column $col"
done

printf '%s\n' \
  "GeographicLib 2.7: ok" \
  "PROJ 9.9.0: ok" \
  "H3 C 4.5.0: ok" \
  "S2 C++ 0.13.1: ok" \
  "NOAA WMM2025 C: ok" \
  "NGS HTDP 3.6.0: ok" \
  "WhiteboxTools 2.4.0: ok (D8 code for east on a plane: $wbt_dir)" \
  "RichDEM 2.3.1: ok (D8 code for east on a plane: $rd_dir)"
