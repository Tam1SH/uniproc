use std::fs::File;
use std::io::{self, BufWriter};
use std::path::Path;

/// Writes `pixels` (top-down, 4 bytes/px, RGBA order) as a PNG file.
pub fn write_png(path: &Path, width: u32, height: u32, pixels_rgba: &[u8]) -> io::Result<()> {
    let expected_len = (width as usize) * (height as usize) * 4;
    if pixels_rgba.len() != expected_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "context2: pixel buffer is {} bytes, expected {expected_len} for {width}x{height}",
                pixels_rgba.len()
            ),
        ));
    }

    let file = File::create(path)?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder
        .write_header()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    writer
        .write_image_data(pixels_rgba)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}
