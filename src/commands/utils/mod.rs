use image::codecs::jpeg::JpegEncoder;

pub mod screen;
pub mod streamer;

pub fn encode_jpeg(rgb_data: &[u8], width: i32, height: i32) -> Option<Vec<u8>> {
    let mut buffer = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut buffer, 100);

    encoder.encode(&rgb_data, width as u32, height as u32, image::ExtendedColorType::Rgb8).ok()?;

    Some(buffer)
}
