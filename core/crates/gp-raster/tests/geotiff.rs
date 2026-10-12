use std::io::Cursor;

use gp_raster::geotiff::{
    Error, RasterType, Transform, decode, decode_file, inspect, inspect_file, same_grid,
};
use tiff::encoder::{TiffEncoder, colortype};
use tiff::tags::Tag;

fn fixture(scale: f64, epsg: u16) -> Vec<u8> {
    fixture_with_orientation(scale, epsg, 1)
}

fn fixture_with_orientation(scale: f64, epsg: u16, orientation: u16) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    {
        let mut encoder = TiffEncoder::new(&mut out).unwrap();
        let mut image = encoder.new_image::<colortype::Gray32Float>(2, 2).unwrap();
        image
            .encoder()
            .write_tag(Tag::ModelPixelScaleTag, &[scale, 20.0, 0.0][..])
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
            .write_tag(
                Tag::GeoKeyDirectoryTag,
                &[
                    1_u16, 1, 0, 3, 1024, 0, 1, 1, 1025, 0, 1, 1, 3072, 0, 1, epsg,
                ][..],
            )
            .unwrap();
        image.encoder().write_tag(Tag::GdalNodata, "-9999").unwrap();
        image
            .encoder()
            .write_tag(Tag::Orientation, orientation)
            .unwrap();
        image.write_data(&[1.0_f32, -9999.0, 3.5, 4.0]).unwrap();
    }
    out.into_inner()
}

#[test]
fn reads_pixels_nodata_crs_and_geotransform() {
    let bytes = fixture(10.0, 32617);
    let metadata = inspect(&bytes).unwrap();
    assert_eq!(metadata.grid.width, 2);
    assert_eq!(metadata.grid.height, 2);
    assert_eq!(metadata.bands, 1);
    assert_eq!(metadata.nodata, Some(-9999.0));
    assert_eq!(metadata.grid.epsg, Some(32617));
    assert_eq!(metadata.grid.raster_type, Some(RasterType::PixelIsArea));
    assert_eq!(
        metadata.grid.transform,
        Some(Transform {
            x0: 500_000.0,
            dx: 10.0,
            rx: 0.0,
            y0: 4_500_000.0,
            ry: 0.0,
            dy: -20.0,
        })
    );

    let raster = decode(&bytes, 4).unwrap();
    assert_eq!(raster.samples, [1.0, -9999.0, 3.5, 4.0]);
    assert_eq!(raster.valid, [true, false, true, true]);
}

#[test]
fn checks_the_sample_limit_before_allocating_pixels() {
    let error = decode(&fixture(10.0, 32617), 3).unwrap_err();
    assert_eq!(
        error,
        Error::LimitExceeded {
            samples: 4,
            limit: 3
        }
    );
}

#[test]
fn reads_the_same_bytes_through_a_private_file_handle() {
    let handle = "geotiff-test-32617";
    gp_base::files::put(handle, &fixture(10.0, 32617)).unwrap();
    assert_eq!(inspect_file(handle).unwrap().grid.epsg, Some(32617));
    assert_eq!(
        decode_file(handle, 4).unwrap().samples,
        [1.0, -9999.0, 3.5, 4.0]
    );
    gp_base::files::remove(handle);
    assert!(
        matches!(inspect_file(handle), Err(Error::Invalid(message)) if message.contains("no longer loaded"))
    );
}

#[test]
fn refuses_an_orientation_that_would_move_pixels_off_the_georeferenced_grid() {
    let error = inspect(&fixture_with_orientation(10.0, 32617, 4)).unwrap_err();
    assert!(matches!(error, Error::Invalid(message) if message.contains("orientation 4")));
}

#[test]
fn grid_mismatch_names_the_difference_and_carries_both_grids() {
    let left = inspect(&fixture(10.0, 32617)).unwrap().grid;
    let right = inspect(&fixture(30.0, 32618)).unwrap().grid;
    let mismatch = same_grid(&left, &right).unwrap_err();
    assert_eq!(mismatch.reason, "CRS");
    assert_eq!(*mismatch.left, left);
    assert_eq!(*mismatch.right, right);

    let same = inspect(&fixture(10.0, 32617)).unwrap().grid;
    same_grid(&left, &same).unwrap();
    let mut moved = same;
    moved.transform.as_mut().unwrap().x0 += 1.0;
    assert_eq!(same_grid(&left, &moved).unwrap_err().reason, "geotransform");
}
