use gp_raster::geotiff::{Grid, Metadata, Raster, RasterType, Transform};
use gp_raster::processing::{Error, normalized_difference};

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
