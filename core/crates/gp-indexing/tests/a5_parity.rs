//! Differential parity with the pinned a5-js 0.10.1 reference.

use a5::{
    LonLat, cell_area, cell_to_boundary, cell_to_children, cell_to_lonlat, cell_to_parent,
    get_resolution, lonlat_to_cell,
};
use gp_indexing::REGISTRY;
use serde_json::Value;

fn near(actual: f64, expected: f64, tolerance: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: {actual} != {expected}"
    );
}

#[test]
fn matches_a5_js_0_10_1() {
    let fixture: Value = serde_json::from_str(include_str!("data/a5_parity.json")).unwrap();
    assert_eq!(fixture["reference"], "a5-js 0.10.1");
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 120);

    for row in rows {
        let lon = row["lon"].as_f64().unwrap();
        let lat = row["lat"].as_f64().unwrap();
        let requested = row["requestedResolution"].as_i64().unwrap() as i32;
        let expected = u64::from_str_radix(row["cell"].as_str().unwrap(), 16).unwrap();
        let cell = lonlat_to_cell(LonLat::new(lon, lat), requested).unwrap();
        assert_eq!(cell, expected, "point {lon},{lat} r{requested}");

        let resolution = get_resolution(cell);
        assert_eq!(resolution, row["resolution"].as_i64().unwrap() as i32);
        let center = cell_to_lonlat(cell).unwrap();
        near(
            center.longitude(),
            row["center"][0].as_f64().unwrap(),
            1e-10,
            "center lon",
        );
        near(
            center.latitude(),
            row["center"][1].as_f64().unwrap(),
            1e-10,
            "center lat",
        );
        near(
            cell_area(resolution),
            row["area"].as_f64().unwrap(),
            row["area"].as_f64().unwrap() * 1e-14,
            "area",
        );

        let boundary = cell_to_boundary(cell, None).unwrap();
        let expected_boundary = row["boundary"].as_array().unwrap();
        assert_eq!(
            boundary.len(),
            expected_boundary.len(),
            "boundary length for {:016x}",
            cell
        );
        for (point, expected_point) in boundary.iter().zip(expected_boundary) {
            near(
                point.longitude(),
                expected_point[0].as_f64().unwrap(),
                1e-9,
                "boundary lon",
            );
            near(
                point.latitude(),
                expected_point[1].as_f64().unwrap(),
                1e-9,
                "boundary lat",
            );
        }

        let parent = cell_to_parent(cell, None).unwrap();
        assert_eq!(format!("{parent:016x}"), row["parent"].as_str().unwrap());
        if resolution < 30 {
            let children: Vec<String> = cell_to_children(cell, None)
                .unwrap()
                .into_iter()
                .map(|child| format!("{child:016x}"))
                .collect();
            assert_eq!(
                children,
                serde_json::from_value::<Vec<String>>(row["children"].clone()).unwrap()
            );
        }

        let response: Value = serde_json::from_str(&REGISTRY.invoke(
            "indexing.a5.lon-lat-to-cell",
            &format!(r#"{{"lat":{lat},"lon":{lon},"resolution":{requested}}}"#),
        ))
        .unwrap();
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["result"]["cell"], format!("{cell:016x}"));
    }
}
