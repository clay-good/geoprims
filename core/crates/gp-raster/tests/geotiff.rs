use std::io::Cursor;

use gp_raster::geotiff::{
    Error, RasterType, Transform, Window, decode, decode_file, decode_file_window, decode_window,
    inspect, inspect_file, same_grid,
};
use tiff::encoder::{TiffEncoder, colortype};
use tiff::tags::Tag;

fn fixture(scale: f64, epsg: u16) -> Vec<u8> {
    fixture_with_orientation(scale, epsg, 1)
}

fn striped_fixture() -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    {
        let mut encoder = TiffEncoder::new(&mut out).unwrap();
        let mut image = encoder.new_image::<colortype::Gray32Float>(4, 4).unwrap();
        image.rows_per_strip(2).unwrap();
        image
            .encoder()
            .write_tag(Tag::ModelPixelScaleTag, &[10.0, 20.0, 0.0][..])
            .unwrap();
        image
            .encoder()
            .write_tag(
                Tag::ModelTiepointTag,
                &[0.0, 0.0, 0.0, 500_000.0, 4_500_000.0, 0.0][..],
            )
            .unwrap();
        image.encoder().write_tag(Tag::GdalNodata, "-9999").unwrap();
        image
            .write_data(&[
                1.0_f32, 2.0, 3.0, 4.0, 5.0, -9999.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0,
                15.0, 16.0,
            ])
            .unwrap();
    }
    out.into_inner()
}

fn tiled_fixture(planar_configuration: u16) -> Vec<u8> {
    fn entry(out: &mut Vec<u8>, tag: u16, field_type: u16, count: u32, value: u32) {
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&field_type.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&value.to_le_bytes());
    }

    const TILE_OFFSETS_OFFSET: u32 = 146;
    const TILE_BYTE_COUNTS_OFFSET: u32 = 154;
    const FIRST_TILE_OFFSET: u32 = 162;
    const SECOND_TILE_OFFSET: u32 = FIRST_TILE_OFFSET + 256;
    let mut out = Vec::with_capacity(674);
    out.extend_from_slice(b"II");
    out.extend_from_slice(&42_u16.to_le_bytes());
    out.extend_from_slice(&8_u32.to_le_bytes());
    out.extend_from_slice(&11_u16.to_le_bytes());
    entry(&mut out, 256, 4, 1, 32); // ImageWidth
    entry(&mut out, 257, 4, 1, 16); // ImageLength
    entry(&mut out, 258, 3, 1, 8); // BitsPerSample
    entry(&mut out, 259, 3, 1, 1); // Compression
    entry(&mut out, 262, 3, 1, 1); // PhotometricInterpretation
    entry(&mut out, 277, 3, 1, 1); // SamplesPerPixel
    entry(&mut out, 284, 3, 1, u32::from(planar_configuration));
    entry(&mut out, 322, 4, 1, 16); // TileWidth
    entry(&mut out, 323, 4, 1, 16); // TileLength
    entry(&mut out, 324, 4, 2, TILE_OFFSETS_OFFSET);
    entry(&mut out, 325, 4, 2, TILE_BYTE_COUNTS_OFFSET);
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(&FIRST_TILE_OFFSET.to_le_bytes());
    out.extend_from_slice(&SECOND_TILE_OFFSET.to_le_bytes());
    out.extend_from_slice(&256_u32.to_le_bytes());
    out.extend_from_slice(&256_u32.to_le_bytes());
    for tile_x in [0_u32, 16] {
        for y in 0_u32..16 {
            for x in tile_x..tile_x + 16 {
                out.push((y * 32 + x) as u8);
            }
        }
    }
    out
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
fn decodes_only_the_strips_intersecting_a_window() {
    let window = Window {
        x: 1,
        y: 1,
        width: 2,
        height: 2,
    };
    let raster = decode_window(&striped_fixture(), window, 8).unwrap();
    assert_eq!(raster.samples, [-9999.0, 7.0, 10.0, 11.0]);
    assert_eq!(raster.valid, [false, true, true, true]);
    assert_eq!(raster.metadata.grid.width, 2);
    assert_eq!(raster.metadata.grid.height, 2);
    assert_eq!(
        raster.metadata.grid.transform,
        Some(Transform {
            x0: 500_010.0,
            dx: 10.0,
            rx: 0.0,
            y0: 4_499_980.0,
            ry: 0.0,
            dy: -20.0,
        })
    );
}

#[test]
fn assembles_a_window_across_tiled_chunks() {
    let raster = decode_window(
        &tiled_fixture(1),
        Window {
            x: 15,
            y: 14,
            width: 2,
            height: 2,
        },
        256,
    )
    .unwrap();
    assert_eq!(raster.samples, [207.0, 208.0, 239.0, 240.0]);
    assert_eq!(raster.valid, [true; 4]);
}

#[test]
fn refuses_planar_storage_instead_of_dropping_bands() {
    let error = inspect(&tiled_fixture(2)).unwrap_err();
    assert!(matches!(error, Error::Invalid(message) if message.contains("Planar")));
}

#[test]
fn validates_window_bounds_and_sample_limit_before_allocating() {
    let bytes = striped_fixture();
    let out_of_bounds = decode_window(
        &bytes,
        Window {
            x: 3,
            y: 3,
            width: 2,
            height: 1,
        },
        8,
    )
    .unwrap_err();
    assert!(matches!(out_of_bounds, Error::Invalid(message) if message.contains("inside")));
    assert_eq!(
        decode_window(
            &bytes,
            Window {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
            3,
        )
        .unwrap_err(),
        Error::LimitExceeded {
            samples: 4,
            limit: 3,
        }
    );
    assert_eq!(
        decode_window(
            &bytes,
            Window {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            1,
        )
        .unwrap_err(),
        Error::LimitExceeded {
            samples: 8,
            limit: 1,
        }
    );
}

#[test]
fn decodes_a_window_through_a_private_file_handle() {
    let handle = "geotiff-window-test";
    gp_base::files::put(handle, &striped_fixture()).unwrap();
    let raster = decode_file_window(
        handle,
        Window {
            x: 2,
            y: 2,
            width: 2,
            height: 1,
        },
        8,
    )
    .unwrap();
    assert_eq!(raster.samples, [11.0, 12.0]);
    gp_base::files::remove(handle);
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
