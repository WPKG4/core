use anyhow::Result;
use image::codecs::jpeg::JpegEncoder;
use scap::capturer::{Capturer, Options};
use scap::frame::Frame;

pub fn fetch_screenshot() -> Result<Vec<u8>> {
    if !scap::is_supported() {
        return Err(anyhow::anyhow!("Platform not supported!"));
    }

    if !scap::has_permission() && !scap::request_permission() {
        return Err(anyhow::anyhow!("Permissions are not sufficient to capture screen data"));
    }

    let options = Options {
        fps: 60,
        show_cursor: true,
        output_type: scap::frame::FrameType::BGRAFrame,
        ..Default::default()
    };

    let mut recorder = Capturer::build(options)?;

    recorder.start_capture();

    let jpeg_data = match process_frame(recorder.get_next_frame()?) {
        Some(jpeg_data) => jpeg_data,
        None => return Err(anyhow::anyhow!("Could not encode frame into Jpeg!")),
    };

    recorder.stop_capture();

    return Ok(jpeg_data)
}

fn process_frame(frame: Frame) -> Option<Vec<u8>> {
    match frame {
        Frame::BGRA(f) => bgra_to_jpg(&f),
        Frame::BGR0(f) => bgr0_to_jpg(&f),
        Frame::BGRx(f) => bgrx_to_jpg(&f),
        Frame::XBGR(f) => xbgr_to_jpg(&f),
        _ => None,
    }
}

fn bgra_to_jpg(frame: &scap::frame::BGRAFrame) -> Option<Vec<u8>>  {
    let rgb_data: Vec<u8> = frame.data.chunks_exact(4)
        .flat_map(|bgra| [bgra[2], bgra[1], bgra[0]])
        .collect();

    encode_jpeg(&rgb_data, frame.width, frame.height)
}

fn bgr0_to_jpg(frame: &scap::frame::BGRFrame) -> Option<Vec<u8>> {
    let rgb_data: Vec<u8> = frame.data.chunks_exact(4)
        .flat_map(|bgr0| [bgr0[2], bgr0[1], bgr0[0]])
        .collect();

    encode_jpeg(&rgb_data, frame.width, frame.height)
}

fn bgrx_to_jpg(frame: &scap::frame::BGRxFrame) -> Option<Vec<u8>> {
    let rgb_data: Vec<u8> = frame.data.chunks_exact(4)
        .flat_map(|bgrx| [bgrx[2], bgrx[1], bgrx[0]])
        .collect();

    encode_jpeg(&rgb_data, frame.width, frame.height)
}

fn xbgr_to_jpg(frame: &scap::frame::XBGRFrame) -> Option<Vec<u8>> {
    let rgb_data: Vec<u8> = frame.data.chunks_exact(4)
        .flat_map(|xbgr| [xbgr[3], xbgr[2], xbgr[1]])
        .collect();

    encode_jpeg(&rgb_data, frame.width, frame.height)
}

fn encode_jpeg(rgb_data: &[u8], width: i32, height: i32) -> Option<Vec<u8>> {
    let mut buffer = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut buffer, 100);
    
    encoder.encode(
        &rgb_data,
        width as u32,
        height as u32,
        image::ExtendedColorType::Rgb8
    ).ok()?;

    Some(buffer)
}