//! Image intake for recipes: sniff, decode, resize to at most 800 px, re-encode (arch/Meals.md §4.1).

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::webp::WebPEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, ImageReader, Limits};
use sha2::{Digest, Sha256};

use crate::error::{MealsError, Result};

/// Largest accepted download or upload.
pub const MAX_INPUT_BYTES: usize = 5 * 1024 * 1024;
/// Largest stored image (the `recipe_image` CHECK).
pub const MAX_STORED_BYTES: usize = 512 * 1024;
/// Longest side after resizing.
pub const MAX_SIDE: u32 = 800;

#[derive(Debug, Clone, PartialEq)]
pub struct StoredImage {
    pub mime: &'static str,
    pub bytes: Vec<u8>,
    pub sha256: String,
}

/// The mime type by magic number: png, jpeg or webp.
pub fn sniff_image(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

fn err(m: &str) -> MealsError {
    MealsError::Image(m.to_string())
}

fn webp(img: &DynamicImage) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let enc = WebPEncoder::new_lossless(&mut out);
    let res = if img.color().has_alpha() {
        img.to_rgba8().write_with_encoder(enc)
    } else {
        img.to_rgb8().write_with_encoder(enc)
    };
    res.ok().map(|()| out)
}

fn jpeg(img: &DynamicImage, quality: u8) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let enc = JpegEncoder::new_with_quality(&mut out, quality);
    img.to_rgb8().write_with_encoder(enc).ok().map(|()| out)
}

fn shrink(img: &DynamicImage, side: u32) -> DynamicImage {
    if img.width().max(img.height()) > side {
        img.resize(side, side, FilterType::Lanczos3)
    } else {
        img.clone()
    }
}

/// Decodes, resizes to at most [`MAX_SIDE`] and encodes as WebP. A photo that is too big as
/// lossless WebP falls back to JPEG at lower qualities, then to a smaller size.
pub fn process(bytes: &[u8]) -> Result<StoredImage> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(err("larger than 5 MB"));
    }
    let mime = sniff_image(bytes).ok_or_else(|| err("not a png, jpeg or webp image"))?;
    let format = match mime {
        "image/png" => ImageFormat::Png,
        "image/jpeg" => ImageFormat::Jpeg,
        _ => ImageFormat::WebP,
    };
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(10_000);
    limits.max_image_height = Some(10_000);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let img = reader
        .decode()
        .map_err(|_| err("the image cannot be decoded"))?;
    let mut side = MAX_SIDE;
    for _ in 0..4 {
        let cur = shrink(&img, side);
        if let Some(b) = webp(&cur)
            && b.len() <= MAX_STORED_BYTES
        {
            return Ok(done("image/webp", b));
        }
        for q in [85u8, 70, 55] {
            if let Some(b) = jpeg(&cur, q)
                && b.len() <= MAX_STORED_BYTES
            {
                return Ok(done("image/jpeg", b));
            }
        }
        side = side * 3 / 4;
    }
    Err(err("the image is too large to store"))
}

fn done(mime: &'static str, bytes: Vec<u8>) -> StoredImage {
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    StoredImage {
        mime,
        bytes,
        sha256,
    }
}
