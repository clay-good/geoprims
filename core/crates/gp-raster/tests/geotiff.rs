use std::io::{self, Cursor, Read, Seek, SeekFrom};

use gp_raster::geotiff::{
    CrsType, Error, Raster, RasterType, Transform, Window, decode, decode_file, decode_file_window,
    decode_window, decode_window_reader, encode, encode_export_tile, inspect, inspect_file,
    same_grid, tiled_export_plan,
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

fn sparse_tiled_fixture() -> Vec<u8> {
    const FIRST_TILE: usize = 100_000;
    const SECOND_TILE: usize = 200_000;
    let compact = tiled_fixture(1);
    let mut sparse = compact[..162].to_vec();
    sparse[146..150].copy_from_slice(&(FIRST_TILE as u32).to_le_bytes());
    sparse[150..154].copy_from_slice(&(SECOND_TILE as u32).to_le_bytes());
    sparse.resize(SECOND_TILE + 256, 0);
    sparse[FIRST_TILE..FIRST_TILE + 256].copy_from_slice(&compact[162..418]);
    sparse[SECOND_TILE..SECOND_TILE + 256].copy_from_slice(&compact[418..674]);
    sparse
}

struct RefuseRange {
    inner: Cursor<Vec<u8>>,
    start: u64,
    end: u64,
}

impl Read for RefuseRange {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let position = self.inner.position();
        let requested_end = position.saturating_add(buffer.len() as u64);
        if position < self.end && requested_end > self.start {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "test blocked an unselected tile",
            ));
        }
        self.inner.read(buffer)
    }
}

impl Seek for RefuseRange {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
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
    assert_eq!(metadata.grid.crs_type, Some(CrsType::Projected));
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
fn exported_result_round_trips_pixels_nodata_crs_and_transform() {
    let source = decode(&fixture(10.0, 32617), 4).unwrap();
    let encoded = encode(&source).unwrap();
    let output = decode(&encoded, 4).unwrap();
    assert_eq!(output, source);
}

#[test]
fn sequential_bigtiff_tiles_round_trip_pixels_nodata_crs_and_transform() {
    let mut source = decode(&fixture(10.0, 32617), 4).unwrap();
    source.metadata.grid.width = 3;
    source.metadata.grid.height = 2;
    source.samples = vec![1.0, 2.0, 3.0, 4.0, -9999.0, 6.0];
    source.valid = vec![true, true, true, true, false, true];
    let plan = tiled_export_plan(&source.metadata, 2, 2).unwrap();
    assert_eq!((plan.tiles_across, plan.tiles_down), (2, 1));

    let left = Raster {
        metadata: gp_raster::geotiff::Metadata {
            grid: gp_raster::geotiff::Grid {
                width: 2,
                height: 2,
                ..source.metadata.grid.clone()
            },
            bands: 1,
            nodata: source.metadata.nodata,
        },
        samples: vec![1.0, 2.0, 4.0, -9999.0],
        valid: vec![true, true, true, false],
    };
    let right = Raster {
        metadata: gp_raster::geotiff::Metadata {
            grid: gp_raster::geotiff::Grid {
                width: 1,
                height: 2,
                ..source.metadata.grid.clone()
            },
            bands: 1,
            nodata: source.metadata.nodata,
        },
        samples: vec![3.0, 6.0],
        valid: vec![true, true],
    };
    let mut bytes = plan.header.clone();
    bytes.extend(encode_export_tile(&left, &plan).unwrap());
    bytes.extend(encode_export_tile(&right, &plan).unwrap());
    assert_eq!(bytes.len() as u64, plan.file_size);
    let output = decode(&bytes, 6).unwrap();
    assert_eq!(output, source);
}

#[test]
fn export_keeps_geographic_epsg_keys_distinct_from_projected_ones() {
    let mut source = decode(&fixture(0.01, 32617), 4).unwrap();
    source.metadata.grid.epsg = Some(4326);
    source.metadata.grid.crs_type = Some(CrsType::Geographic);
    let output = decode(&encode(&source).unwrap(), 4).unwrap();
    assert_eq!(output.metadata.grid.epsg, Some(4326));
    assert_eq!(output.metadata.grid.crs_type, Some(CrsType::Geographic));
}

#[test]
fn export_uses_nan_when_new_invalid_pixels_have_no_source_sentinel() {
    let mut source = decode(&fixture(10.0, 32617), 4).unwrap();
    source.metadata.nodata = None;
    source.valid[1] = false;
    let output = decode(&encode(&source).unwrap(), 4).unwrap();
    assert!(output.metadata.nodata.unwrap().is_nan());
    assert!(output.samples[1].is_nan());
    assert_eq!(output.valid, [true, false, true, true]);
}

#[test]
fn export_refuses_a_nodata_sentinel_that_collides_with_valid_output() {
    let mut source = decode(&fixture(10.0, 32617), 4).unwrap();
    source.metadata.nodata = Some(1.0);
    let error = encode(&source).unwrap_err();
    assert!(matches!(error, Error::Invalid(message) if message.contains("sentinel")));
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
fn a_window_does_not_read_unselected_tile_pixels() {
    let reader = RefuseRange {
        inner: Cursor::new(tiled_fixture(1)),
        start: 418,
        end: 674,
    };
    let raster = decode_window_reader(
        reader,
        Window {
            x: 14,
            y: 14,
            width: 2,
            height: 2,
        },
        256,
    )
    .unwrap();
    assert_eq!(raster.samples, [206.0, 207.0, 238.0, 239.0]);
}

#[test]
fn sparse_file_retries_request_only_the_selected_tile_range() {
    let handle = "sparse-tiled-cog";
    let bytes = sparse_tiled_fixture();
    gp_base::files::open_sparse(handle, bytes.len() as u64).unwrap();
    let window = Window {
        x: 16,
        y: 14,
        width: 2,
        height: 2,
    };
    let mut requested = Vec::new();
    let raster = loop {
        match decode_file_window(handle, window, 256) {
            Ok(raster) => break raster,
            Err(_) => {
                let missing = gp_base::files::missing_range().expect("missing range");
                requested.push(missing.offset);
                let start = missing.offset as usize;
                let end = start + missing.length;
                gp_base::files::put_range(handle, missing.offset, &bytes[start..end]).unwrap();
            }
        }
    };
    assert_eq!(requested, [0, 196_608]);
    assert_eq!(raster.samples, [208.0, 209.0, 240.0, 241.0]);
    gp_base::files::remove(handle);
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
