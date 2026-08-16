use image::imageops::FilterType::Lanczos3;
use image::DynamicImage;

const JPEG_QUALITY: u8 = 90;
const MAX_SHORT_SIDE: u32 = 1125;
/// PNG/GIF sources: constrain longest side (like PIL thumbnail).
/// Screenshots/text/UI benefit from keeping more of the original
/// frame after Retina downsample.
const MAX_LONG_SIDE: u32 = 1800;

#[derive(Debug, Clone)]
pub struct CompressedImage {
    pub data: Vec<u8>,
    pub mime_type: String,
}

fn source_is_gif(raw_bytes: &[u8]) -> bool {
    raw_bytes.starts_with(b"GIF8")
}

fn source_is_png(raw_bytes: &[u8]) -> bool {
    raw_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
}

fn resize_to_max_short_side(img: &DynamicImage) -> DynamicImage {
    let (w, h) = (img.width(), img.height());
    let shortest = w.min(h);
    if shortest > MAX_SHORT_SIDE {
        let scale = MAX_SHORT_SIDE as f64 / shortest as f64;
        img.resize(
            (w as f64 * scale).max(1.0) as u32,
            (h as f64 * scale).max(1.0) as u32,
            Lanczos3,
        )
    } else {
        img.clone()
    }
}

fn resize_to_max_long_side(img: &DynamicImage) -> DynamicImage {
    let (w, h) = (img.width(), img.height());
    let longest = w.max(h);
    if longest > MAX_LONG_SIDE {
        let scale = MAX_LONG_SIDE as f64 / longest as f64;
        img.resize(
            (w as f64 * scale).max(1.0) as u32,
            (h as f64 * scale).max(1.0) as u32,
            Lanczos3,
        )
    } else {
        img.clone()
    }
}

pub fn compress_image(raw_bytes: &[u8]) -> Result<CompressedImage, String> {
    if source_is_gif(raw_bytes) {
        return Ok(CompressedImage { data: raw_bytes.to_vec(), mime_type: "image/gif".to_string() });
    }
    let img = image::load_from_memory(raw_bytes)
        .map_err(|e| format!("Cannot decode image: {e}"))?;
    if source_is_png(raw_bytes) {
        // PNG source (photo or screenshot) → JPEG Q90
        let resized = resize_to_max_long_side(&img);
        let data = encode_jpeg(&resized, JPEG_QUALITY)?;
        return Ok(CompressedImage { data, mime_type: "image/jpeg".to_string() });
    }
    let resized = resize_to_max_short_side(&img);
    let data = encode_jpeg(&resized, JPEG_QUALITY)?;
    Ok(CompressedImage { data, mime_type: "image/jpeg".to_string() })
}

fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    let mut buffer = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
    img.write_with_encoder(encoder)
        .map_err(|e| format!("JPEG encode: {e}"))?;
    Ok(buffer)
}


