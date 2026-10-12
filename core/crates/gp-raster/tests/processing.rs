use gp_raster::geotiff::{Grid, Metadata, Raster, RasterType, Transform};
use std::io::Cursor;

use gp_raster::processing::{Error, normalized_difference, normalized_difference_scene};
use tiff::encoder::{TiffEncoder, colortype};
use tiff::tags::Tag;

fn grid(width: u32) -> Grid {
    Grid {
        width,
        height: 2,
        transform: Some(Transform {
            x0: 500_000.0,
            dx: 10.0,
            rx: 0.0,
            y0: 4_500_000.0,
            ry: 0.0,
            dy: -10.0,
        }),
        epsg: Some(32617),
        raster_type: Some(RasterType::PixelIsArea),
    }
}

fn raster(grid: Grid, bands: u16, nodata: Option<f64>, samples: Vec<f64>) -> Raster {
    let valid = samples
        .iter()
        .map(|sample| nodata != Some(*sample))
        .collect();
    Raster {
        metadata: Metadata {
            grid,
            bands,
            nodata,
        },
        samples,
        valid,
    }
}

fn tiff_fixture(width: u32, values: &[f32], nodata: f64) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    let nodata = nodata.to_string();
    {
        let mut encoder = TiffEncoder::new(&mut out).unwrap();
        let mut image = encoder
            .new_image::<colortype::Gray32Float>(width, 4)
            .unwrap();
        image.rows_per_strip(2).unwrap();
        image
            .encoder()
            .write_tag(Tag::ModelPixelScaleTag, &[10.0, 10.0, 0.0][..])
            .unwrap();
        image
            .encoder()
            .write_tag(
                Tag::ModelTiepointTag,
                &[0.0, 0.0, 0.0, 500_000.0, 4_500_000.0, 0.0][..],
            )
            .unwrap();
        image
            .encoder()
            .write_tag(Tag::GdalNodata, nodata.as_str())
            .unwrap();
        image.write_data(values).unwrap();
    }
    out.into_inner()
}

#[test]
fn processes_interleaved_bands_and_propagates_nodata_and_zero_denominators() {
    let left = raster(
        grid(2),
        2,
        Some(-9999.0),
        vec![9.0, 0.8, 8.0, -9999.0, 7.0, 0.0, 6.0, -0.5],
    );
    let right = raster(
        grid(2),
        2,
        Some(-32768.0),
        vec![1.0, 9.0, 0.2, 8.0, 0.0, 7.0, 0.5, 6.0],
    );
    let output = normalized_difference(&left, 1, &right, 0).unwrap();
    assert_eq!(
        output.samples,
        [(0.8 - 1.0) / (0.8 + 1.0), -9999.0, -9999.0, -9999.0]
    );
    assert_eq!(output.valid, [true, false, false, false]);
    assert_eq!(output.metadata.grid, left.metadata.grid);
    assert_eq!(output.metadata.bands, 1);
    assert_eq!(output.metadata.nodata, Some(-9999.0));
}

#[test]
fn computes_each_pixel_without_changing_its_grid() {
    let left = raster(grid(2), 1, None, vec![0.8, 0.6, 0.4, 0.2]);
    let right = raster(grid(2), 1, None, vec![0.2, 0.2, 0.1, 0.1]);
    let output = normalized_difference(&left, 0, &right, 0).unwrap();
    assert_eq!(
        output.samples,
        [
            (0.8 - 0.2) / (0.8 + 0.2),
            (0.6 - 0.2) / (0.6 + 0.2),
            (0.4 - 0.1) / (0.4 + 0.1),
            (0.2 - 0.1) / (0.2 + 0.1),
        ]
    );
    assert_eq!(output.valid, [true; 4]);
    assert_eq!(output.metadata.grid, left.metadata.grid);
}

#[test]
fn grid_mismatch_carries_both_grids_and_the_reason() {
    let left = raster(grid(2), 1, None, vec![0.8; 4]);
    let right = raster(grid(3), 1, None, vec![0.2; 6]);
    let error = normalized_difference(&left, 0, &right, 0).unwrap_err();
    let Error::GridMismatch(mismatch) = error else {
        panic!("expected grid mismatch");
    };
    assert_eq!(mismatch.reason, "dimensions");
    assert_eq!(*mismatch.left, left.metadata.grid);
    assert_eq!(*mismatch.right, right.metadata.grid);
}

#[test]
fn rejects_a_missing_band_and_an_inconsistent_buffer() {
    let raster = raster(grid(2), 1, None, vec![0.8; 4]);
    assert_eq!(
        normalized_difference(&raster, 1, &raster, 0).unwrap_err(),
        Error::InvalidBand { band: 1, bands: 1 }
    );
    let mut broken = raster.clone();
    broken.samples.pop();
    assert!(matches!(
        normalized_difference(&broken, 0, &raster, 0),
        Err(Error::InvalidRaster(_))
    ));
}

#[test]
fn processes_a_scene_in_bounded_row_major_windows() {
    let mut left = vec![0.8_f32; 16];
    left[9] = -9999.0;
    let right = vec![0.2_f32; 16];
    let mut windows = Vec::new();
    normalized_difference_scene(
        Cursor::new(tiff_fixture(4, &left, -9999.0)),
        0,
        Cursor::new(tiff_fixture(4, &right, -32768.0)),
        0,
        2,
        1,
        8,
        |window, raster| windows.push((window, raster)),
    )
    .unwrap();

    assert_eq!(windows.len(), 8);
    assert_eq!((windows[0].0.x, windows[0].0.y), (0, 0));
    assert_eq!((windows[1].0.x, windows[1].0.y), (2, 0));
    assert_eq!((windows[7].0.x, windows[7].0.y), (2, 3));
    let valid = windows
        .iter()
        .flat_map(|(_, raster)| raster.valid.iter())
        .filter(|valid| **valid)
        .count();
    assert_eq!(valid, 15);
    assert_eq!(windows[4].1.samples[1], -9999.0);
    assert_eq!(windows[4].1.metadata.grid.transform.unwrap().x0, 500_000.0);
    assert_eq!(
        windows[4].1.metadata.grid.transform.unwrap().y0,
        4_499_980.0
    );
    for (_, raster) in &windows {
        for (&sample, &valid) in raster.samples.iter().zip(&raster.valid) {
            if valid {
                assert!((sample - 0.6).abs() < 1e-7);
            }
        }
    }
}

#[test]
fn scene_grid_mismatch_is_reported_before_any_output() {
    let left = vec![0.8_f32; 16];
    let right = vec![0.2_f32; 20];
    let mut called = false;
    let error = normalized_difference_scene(
        Cursor::new(tiff_fixture(4, &left, -9999.0)),
        0,
        Cursor::new(tiff_fixture(5, &right, -9999.0)),
        0,
        2,
        1,
        10,
        |_, _| called = true,
    )
    .unwrap_err();
    assert!(!called);
    assert!(matches!(
        error,
        Error::GridMismatch(mismatch) if mismatch.reason == "dimensions"
    ));
}
