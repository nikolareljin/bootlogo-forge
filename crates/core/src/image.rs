use std::io::Cursor;

use image::{
    codecs::bmp::BmpEncoder, imageops, ExtendedColorType, ImageError as ImgError, RgbImage,
};

#[derive(Debug)]
pub enum ImagePipelineError {
    EmptyInput,
    OversizedInput,
    InvalidResolution,
    Decode(String),
    Encode(String),
}

impl std::fmt::Display for ImagePipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyInput => f.write_str("input image is empty"),
            Self::OversizedInput => f.write_str("input image is too large"),
            Self::InvalidResolution => f.write_str("invalid output resolution"),
            Self::Decode(m) => write!(f, "could not decode image: {}", m),
            Self::Encode(m) => write!(f, "could not encode BMP: {}", m),
        }
    }
}

impl std::error::Error for ImagePipelineError {}

const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;
const MAX_DIMENSION: u32 = 16384;

pub fn render_bmp(input: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ImagePipelineError> {
    if input.is_empty() {
        return Err(ImagePipelineError::EmptyInput);
    }
    if input.len() > MAX_INPUT_BYTES {
        return Err(ImagePipelineError::OversizedInput);
    }
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(ImagePipelineError::InvalidResolution);
    }

    let decoded = image::load_from_memory(input)
        .map_err(|e: ImgError| ImagePipelineError::Decode(e.to_string()))?;
    if decoded.width() == 0 || decoded.height() == 0 {
        return Err(ImagePipelineError::Decode(String::from("zero-size image")));
    }

    let (scaled_w, scaled_h) = fit_dimensions(decoded.width(), decoded.height(), width, height);
    let resized = imageops::resize(
        &decoded.to_rgb8(),
        scaled_w,
        scaled_h,
        imageops::FilterType::Lanczos3,
    );

    let mut canvas = RgbImage::new(width, height);
    let x = i64::from(width.saturating_sub(scaled_w)) / 2;
    let y = i64::from(height.saturating_sub(scaled_h)) / 2;
    imageops::overlay(&mut canvas, &resized, x, y);

    let mut output = Vec::new();
    let mut cursor = Cursor::new(&mut output);
    let mut encoder = BmpEncoder::new(&mut cursor);
    encoder
        .encode(canvas.as_raw(), width, height, ExtendedColorType::Rgb8)
        .map_err(|e| ImagePipelineError::Encode(e.to_string()))?;

    Ok(output)
}

fn fit_dimensions(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> (u32, u32) {
    let scale_w = dst_w as f64 / src_w as f64;
    let scale_h = dst_h as f64 / src_h as f64;
    let scale = if scale_w < scale_h { scale_w } else { scale_h };
    let new_w = ((src_w as f64 * scale).round().max(1.0)) as u32;
    let new_h = ((src_h as f64 * scale).round().max(1.0)) as u32;
    (new_w.min(dst_w).max(1), new_h.min(dst_h).max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{
        codecs::jpeg::JpegEncoder, codecs::png::PngEncoder, DynamicImage, GenericImageView,
        ImageBuffer, ImageEncoder, Rgb,
    };
    use std::io::Cursor;

    fn make_png(width: u32, height: u32, color: [u8; 3]) -> Vec<u8> {
        let buffer = ImageBuffer::from_pixel(width, height, Rgb(color));
        let img = DynamicImage::ImageRgb8(buffer);
        let mut out = Vec::new();
        let mut cursor = Cursor::new(&mut out);
        let encoder = PngEncoder::new(&mut cursor);
        encoder
            .write_image(img.as_bytes(), width, height, ExtendedColorType::Rgb8)
            .unwrap();
        out
    }

    fn make_jpeg(width: u32, height: u32, color: [u8; 3]) -> Vec<u8> {
        let buffer = ImageBuffer::from_pixel(width, height, Rgb(color));
        let img = DynamicImage::ImageRgb8(buffer);
        let mut out = Vec::new();
        let mut cursor = Cursor::new(&mut out);
        let mut encoder = JpegEncoder::new_with_quality(&mut cursor, 90);
        encoder
            .encode(img.as_bytes(), width, height, ExtendedColorType::Rgb8)
            .unwrap();
        out
    }

    fn read_u32_le(bytes: &[u8], off: usize) -> u32 {
        u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap())
    }

    fn read_i32_le(bytes: &[u8], off: usize) -> i32 {
        i32::from_le_bytes(bytes[off..off + 4].try_into().unwrap())
    }

    #[test]
    fn png_round_trip() {
        let input = make_png(4, 4, [200, 10, 50]);
        let output = render_bmp(&input, 64, 48).unwrap();
        assert_eq!(&output[0..2], b"BM");
        assert_eq!(read_u32_le(&output, 10), 54);
        assert_eq!(read_i32_le(&output, 18), 64);
        assert_eq!(read_i32_le(&output, 22), 48);
        assert_eq!(read_u16(&output, 28), 24);
        let decoded = image::load_from_memory(&output).unwrap();
        assert_eq!(decoded.width(), 64);
        assert_eq!(decoded.height(), 48);
        let center = decoded.get_pixel(32, 24);
        assert_eq!(center.0, [200, 10, 50, 255]);
    }

    #[test]
    fn jpeg_round_trip() {
        let input = make_jpeg(8, 6, [10, 200, 30]);
        let output = render_bmp(&input, 80, 60).unwrap();
        let decoded = image::load_from_memory(&output).unwrap();
        assert_eq!(decoded.width(), 80);
        assert_eq!(decoded.height(), 60);
        let center = decoded.get_pixel(40, 30);
        assert!(center.0[1] > 150);
    }
    fn read_u16(bytes: &[u8], off: usize) -> u16 {
        u16::from_le_bytes(bytes[off..off + 2].try_into().unwrap())
    }

    #[test]
    fn empty_input() {
        assert!(matches!(
            render_bmp(&[], 10, 10),
            Err(ImagePipelineError::EmptyInput)
        ));
    }

    #[test]
    fn corrupt_input() {
        let bad = vec![1u8, 2, 3, 4, 5];
        assert!(matches!(
            render_bmp(&bad, 10, 10),
            Err(ImagePipelineError::Decode(_))
        ));
    }

    #[test]
    fn truncated_input() {
        let input = make_png(4, 4, [1, 2, 3]);
        let truncated = &input[..input.len() / 2];
        assert!(matches!(
            render_bmp(truncated, 10, 10),
            Err(ImagePipelineError::Decode(_))
        ));
    }

    #[test]
    fn invalid_resolution() {
        let input = make_png(4, 4, [1, 2, 3]);
        assert!(matches!(
            render_bmp(&input, 0, 10),
            Err(ImagePipelineError::InvalidResolution)
        ));
        assert!(matches!(
            render_bmp(&input, 10, 0),
            Err(ImagePipelineError::InvalidResolution)
        ));
    }

    #[test]
    fn oversized_input() {
        let input = vec![0u8; 33 * 1024 * 1024];
        assert!(matches!(
            render_bmp(&input, 10, 10),
            Err(ImagePipelineError::OversizedInput)
        ));
    }
}
