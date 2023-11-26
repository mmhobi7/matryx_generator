use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};
use std::thread::{self, sleep};
use std::time;

use image::{DynamicImage, ImageBuffer};
use imageproc::stats::percentile;
use log2::*;
use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;
use v4l::video::Capture;
use v4l::Device;
use v4l::FourCC;
use v4l::{prelude::*, Format};

const CAMERA_FRAME_DELAY: time::Duration = time::Duration::from_millis(500);
const RETRY_DELAY: time::Duration = time::Duration::from_secs(5);
const MAX_ATTEMPTS: i8 = std::i8::MAX - 1;
const DEVICE_MISSING_ERROR: i32 = -1;
const INVALID_BUFFER_PIXEL_FORMAT_ERROR: i32 = -2;

pub fn cam_thread_loop(hists_clone: Arc<AtomicU8>) {
    let mut attempt: i8 = 1;
    let mut cam_thread_ret = cam_thread(hists_clone.clone(), attempt);
    loop {
        match cam_thread_ret {
            Ok(_) => {
                warn!("unreachable");
            }
            Err(e) => {
                warn!("Camera Error: {e:?}");
                if attempt >= MAX_ATTEMPTS {
                    attempt = 1;
                } else {
                    attempt += 1;
                }
                cam_thread_ret = cam_thread(hists_clone.clone(), attempt);
            }
        }
        sleep(RETRY_DELAY);
        error!("cam thread loop slept")
    }
}

fn set_device_format(dev: &mut Device, fourcc: FourCC) -> Result<Format, i32> {
    let mut format = dev.format().map_err(|e| {
        error!("Failed to read format: {}", e);
        DEVICE_MISSING_ERROR
    })?;
    format.fourcc = fourcc;
    dev.set_format(&format).map_err(|e| {
        error!("set format {}", e);
        DEVICE_MISSING_ERROR
    })
}

fn cam_thread(hists_clone: Arc<AtomicU8>, attempt: i8) -> Result<i32, i32> {
    error!("Camera time, Attempt: {}\n", attempt);

    let mut dev = Device::new(0).map_err(|e| {
        error!("Device missing: {}", e);
        DEVICE_MISSING_ERROR
    })?;

    let mut format = set_device_format(&mut dev, FourCC::new(b"RGB3"))?;
    if format.fourcc != FourCC::new(b"RGB3") {
        // fallback to Motion-JPEG
        format = set_device_format(&mut dev, FourCC::new(b"MJPG"))?;
    }

    error!("Active format:\n{}", format);

    error!("starting stream");
    let mut stream = UserptrStream::with_buffers(&mut dev, Type::VideoCapture, 1).map_err(|e| {
        error!("Failed to create buffer stream {}", e);
        DEVICE_MISSING_ERROR
    })?;
    error!("stream started");

    loop {
        let _ = stream.next();
        let (buf, _) = stream.next().map_err(|e| {
            error!("Camera thread dead: {}", e);
            DEVICE_MISSING_ERROR
        })?;
        let data = match &format.fourcc.repr {
            b"RGB3" => buf.to_vec(),
            b"MJPG" => {
                // Decode the JPEG frame to RGB
                let mut decoder = jpeg_decoder::Decoder::new(buf);
                decoder.decode().expect("failed to decode JPEG")
            }
            _ => {
                error!("invalid buffer pixelformat");
                return Err(INVALID_BUFFER_PIXEL_FORMAT_ERROR);
            }
        };
        let img: ImageBuffer<image::Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_raw(format.width, format.height, data).unwrap();
        let luma = DynamicImage::ImageRgb8(img).into_luma8();
        let val = percentile(&luma, 90);
        hists_clone.store(val, Ordering::Relaxed);
        thread::sleep(CAMERA_FRAME_DELAY);
    }
}
