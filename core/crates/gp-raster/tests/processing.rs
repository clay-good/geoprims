use gp_raster::geotiff::{CrsType, Grid, Metadata, Raster, RasterType, Transform, decode};
use std::io::{self, Cursor, Seek, SeekFrom, Write};

use gp_raster::processing::{
    Error, Kernel, Resampling, TargetGrid, index, normalized_difference,
    normalized_difference_file_window, normalized_difference_resampled_window,
    normalized_difference_scene, resample, write_normalized_difference_geotiff,
};
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
        crs_type: Some(CrsType::Projected),
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

#[test]
fn fixed_two_band_kernels_match_the_scalar_equations() {
    let left = raster(grid(2), 1, Some(-9999.0), vec![0.45, -9999.0, 0.61, 1.1]);
    let right = raster(grid(2), 1, None, vec![0.08, 0.08, 0.13, 0.27]);

    let evi2 = index(&left, 0, &right, 0, Kernel::Evi2).unwrap();
    assert!((evi2.samples[0] - 2.5 * (0.45 - 0.08) / (0.45 + 2.4 * 0.08 + 1.0)).abs() < 1e-15);
    assert!(!evi2.valid[1]);
    assert_eq!(evi2.samples[1], -9999.0);

    let savi = index(&left, 0, &right, 0, Kernel::Savi { soil_factor: 0.5 }).unwrap();
    assert!((savi.samples[0] - 0.538_834_951_456_310_6).abs() < 1e-15);

    let dnbr = index(&left, 0, &right, 0, Kernel::Dnbr).unwrap();
    assert!((dnbr.samples[2] - 0.48).abs() < 1e-15);
    assert!(!dnbr.valid[3]);
}

fn tiff_fixture(width: u32, values: &[f32], nodata: f64) -> Vec<u8> {
    tiff_fixture_with_scale(width, values, nodata, 10.0)
}

fn tiff_fixture_with_scale(width: u32, values: &[f32], nodata: f64, scale: f64) -> Vec<u8> {
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
            .write_tag(Tag::ModelPixelScaleTag, &[scale, 10.0, 0.0][..])
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
fn resampled_file_windows_use_the_chosen_grid_and_nodata() {
    let left = tiff_fixture_with_scale(2, &[0.8; 8], -9999.0, 20.0);
    let right = tiff_fixture_with_scale(4, &[0.2; 16], -32768.0, 10.0);
    let coarse = normalized_difference_resampled_window(
        Cursor::new(&left),
        0,
        Cursor::new(&right),
        0,
        gp_raster::geotiff::Window {
            x: 0,
            y: 0,
            width: 2,
            height: 2,
        },
        64,
        Resampling::Nearest,
        TargetGrid::Left,
    )
    .unwrap();
    assert_eq!(coarse.metadata.grid.width, 2);
    assert_eq!(coarse.metadata.grid.transform.unwrap().dx, 20.0);
    assert_eq!(coarse.metadata.nodata, Some(-9999.0));
    assert!(
        coarse
            .samples
            .iter()
            .all(|value| (*value - 0.6).abs() < 1e-7)
    );

    let fine = normalized_difference_resampled_window(
        Cursor::new(left),
        0,
        Cursor::new(right),
        0,
        gp_raster::geotiff::Window {
            x: 0,
            y: 0,
            width: 4,
            height: 2,
        },
        64,
        Resampling::Bilinear,
        TargetGrid::Right,
    )
    .unwrap();
    assert_eq!(fine.metadata.grid.width, 4);
    assert_eq!(fine.metadata.grid.transform.unwrap().dx, 10.0);
    assert_eq!(fine.metadata.nodata, Some(-32768.0));
    assert_eq!(
        fine.valid,
        [false, true, true, false, false, true, true, false]
    );
    for (&sample, &valid) in fine.samples.iter().zip(&fine.valid) {
        if valid {
            assert!((sample - 0.6).abs() < 1e-7);
        } else {
            assert_eq!(sample, -32768.0);
        }
    }
}

#[derive(Debug)]
struct FailingWriter {
    inner: Cursor<Vec<u8>>,
    remaining: usize,
}

impl Write for FailingWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Err(io::Error::other("test output failed"));
        }
        let length = buffer.len().min(self.remaining);
        let written = self.inner.write(&buffer[..length])?;
        self.remaining -= written;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl Seek for FailingWriter {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
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
        |window, raster| {
            windows.push((window, raster));
            Ok(())
        },
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
        |_, _| {
            called = true;
            Ok(())
        },
    )
    .unwrap_err();
    assert!(!called);
    assert!(matches!(
        error,
        Error::GridMismatch(mismatch) if mismatch.reason == "dimensions"
    ));
}

#[test]
fn file_window_checks_the_complete_source_grids_before_decoding_pixels() {
    gp_base::files::clear();
    gp_base::files::put("left-grid", &tiff_fixture(4, &[0.8; 16], -9999.0)).unwrap();
    gp_base::files::put("right-grid", &tiff_fixture(5, &[0.2; 20], -9999.0)).unwrap();
    let error = normalized_difference_file_window(
        "left-grid",
        0,
        "right-grid",
        0,
        gp_raster::geotiff::Window {
            x: 0,
            y: 0,
            width: 2,
            height: 1,
        },
        10,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        Error::GridMismatch(mismatch) if mismatch.reason == "dimensions"
    ));
    gp_base::files::clear();
}

#[test]
fn a_scene_consumer_error_stops_before_the_next_window() {
    let left = vec![0.8_f32; 16];
    let right = vec![0.2_f32; 16];
    let mut calls = 0;
    let error = normalized_difference_scene(
        Cursor::new(tiff_fixture(4, &left, -9999.0)),
        0,
        Cursor::new(tiff_fixture(4, &right, -9999.0)),
        0,
        2,
        1,
        8,
        |_, _| {
            calls += 1;
            Err(Error::InvalidRaster("test stopped output"))
        },
    )
    .unwrap_err();
    assert_eq!(calls, 1);
    assert_eq!(error, Error::InvalidRaster("test stopped output"));
}

#[test]
fn streams_a_complete_georeferenced_result_by_strip() {
    let mut left = vec![0.8_f32; 16];
    left[9] = -9999.0;
    let right = vec![0.2_f32; 16];
    let output = write_normalized_difference_geotiff(
        Cursor::new(tiff_fixture(4, &left, -9999.0)),
        0,
        Cursor::new(tiff_fixture(4, &right, -32768.0)),
        0,
        1,
        8,
        Cursor::new(Vec::new()),
    )
    .unwrap()
    .into_inner();
    let result = decode(&output, 16).unwrap();
    assert_eq!(result.metadata.grid.width, 4);
    assert_eq!(result.metadata.grid.height, 4);
    assert_eq!(result.metadata.grid.transform, grid(4).transform);
    assert_eq!(result.metadata.nodata, Some(-9999.0));
    assert_eq!(result.valid.iter().filter(|valid| **valid).count(), 15);
    for (&sample, &valid) in result.samples.iter().zip(&result.valid) {
        if valid {
            assert!((sample - 0.6).abs() < 1e-7);
        }
    }
}

#[test]
fn streaming_export_returns_the_first_writer_failure() {
    let left = vec![0.8_f32; 16];
    let right = vec![0.2_f32; 16];
    let error = write_normalized_difference_geotiff(
        Cursor::new(tiff_fixture(4, &left, -9999.0)),
        0,
        Cursor::new(tiff_fixture(4, &right, -9999.0)),
        0,
        1,
        8,
        FailingWriter {
            inner: Cursor::new(Vec::new()),
            remaining: 64,
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        Error::Export(geotiff_error) if matches!(&geotiff_error, gp_raster::geotiff::Error::Invalid(message) if message.contains("test output failed"))
    ));
}

#[test]
fn nearest_resampling_repeats_pixels_on_a_finer_area_grid() {
    let source = raster(grid(2), 1, None, vec![1.0, 2.0, 3.0, 4.0]);
    let mut target = grid(4);
    target.height = 4;
    target.transform.as_mut().unwrap().dx = 5.0;
    target.transform.as_mut().unwrap().dy = -5.0;
    let output = resample(&source, target.clone(), Resampling::Nearest, 16).unwrap();
    assert_eq!(
        output.samples,
        [
            1.0, 1.0, 2.0, 2.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0, 3.0, 3.0, 4.0, 4.0,
        ]
    );
    assert_eq!(output.metadata.grid, target);
}

#[test]
fn bilinear_resampling_uses_pixel_centers_and_propagates_nodata() {
    let mut source_grid = grid(2);
    source_grid.raster_type = Some(RasterType::PixelIsPoint);
    let source = raster(
        source_grid.clone(),
        1,
        Some(-9999.0),
        vec![1.0, 2.0, 3.0, 4.0],
    );
    let mut target = source_grid;
    target.width = 1;
    target.height = 1;
    target.transform.as_mut().unwrap().x0 += 5.0;
    target.transform.as_mut().unwrap().y0 -= 5.0;
    let output = resample(&source, target.clone(), Resampling::Bilinear, 1).unwrap();
    assert_eq!(output.samples, [2.5]);
    assert_eq!(output.valid, [true]);

    let mut masked = source;
    masked.valid[3] = false;
    let output = resample(&masked, target, Resampling::Bilinear, 1).unwrap();
    assert_eq!(output.samples, [-9999.0]);
    assert_eq!(output.valid, [false]);
}

#[test]
fn resampling_refuses_other_crs_and_enforces_the_output_cap() {
    let source = raster(grid(2), 1, None, vec![1.0, 2.0, 3.0, 4.0]);
    let mut other_crs = grid(2);
    other_crs.epsg = Some(32618);
    assert!(matches!(
        resample(&source, other_crs, Resampling::Nearest, 4),
        Err(Error::GridMismatch(mismatch)) if mismatch.reason == "CRS"
    ));
    assert_eq!(
        resample(&source, grid(2), Resampling::Nearest, 3).unwrap_err(),
        Error::LimitExceeded {
            samples: 4,
            limit: 3,
        }
    );
}

#[test]
fn resampling_round_trips_a_rotated_grid_exactly() {
    let mut rotated = grid(2);
    rotated.transform = Some(Transform {
        x0: 10.0,
        dx: 2.0,
        rx: 0.5,
        y0: 20.0,
        ry: -0.25,
        dy: -3.0,
    });
    let source = raster(rotated.clone(), 1, None, vec![1.0, 2.0, 3.0, 4.0]);
    for method in [Resampling::Nearest, Resampling::Bilinear] {
        let output = resample(&source, rotated.clone(), method, 4).unwrap();
        assert_eq!(output.samples, source.samples);
        assert_eq!(output.valid, source.valid);
    }
}
