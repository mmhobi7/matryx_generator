
use image::{DynamicImage, ImageBuffer};
use imageproc::stats::percentile;
use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;
use v4l::prelude::*;
use v4l::video::Capture;
use v4l::Device;
use v4l::FourCC;

use log2::*;
use std::time;
use std::{
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    thread::{self, sleep},
    time::Duration,
};

const CAMERA_FRAME_DELAY: time::Duration = time::Duration::from_millis(500);

pub fn cam_thread_loop(hists_clone: Arc<AtomicU8>) {
    let mut attempt: i8 = 1;
    let mut cam_thread_ret = cam_thread(hists_clone.clone(), attempt);
    loop {
        match cam_thread_ret {
            Ok(v) => {
                warn!("unreachable: {v:?}");
            }
            Err(e) => {
                warn!("Camera Error: {e:?}");
                if attempt >= std::i8::MAX {
                    attempt = 1;
                } else {
                    attempt = attempt + 1;
                }
                cam_thread_ret = cam_thread(hists_clone.clone(), attempt);
            }
        }
        sleep(Duration::from_secs(5));
        error!("cam thread loop slept")
    }
}

fn cam_thread(hists_clone: Arc<AtomicU8>, attempt: i8) -> Result<i32, i32> {
    error!("Camera time, Attempt: {}\n", attempt);
    // let mut dev = Device::new(2).unwrap();

    let mut dev = {
        let this = Device::new(0);
        match this {
            Ok(t) => t,
            Err(e) => {
                error!("Device missing: {}", e);
                return Err(-1);
            }
        }
    };

    // Let's say we want to explicitly request another format
    let mut format = {
        let this = dev.format();
        match this {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to read format: {}", e);
                return Err(-1);
            }
        }
    };
    error!("format set");
    format.fourcc = FourCC::new(b"RGB3");
    // format = dev.set_format(&format).unwrap();
    format = {
        let this = dev.set_format(&format);
        match this {
            Ok(t) => t,
            Err(e) => {
                error!("set format {}", e);
                return Err(-1);
            }
        }
    };

    if format.fourcc != FourCC::new(b"RGB3") {
        // fallback to Motion-JPEG
        format.fourcc = FourCC::new(b"MJPG");
        // format = dev.set_format(&format).unwrap();
        format = {
            let this = dev.set_format(&format);
            match this {
                Ok(t) => t,
                Err(e) => {
                    error!("set format {}", e);
                    return Err(-1);
                }
            }
        };
    }

    error!("Active format:\n{}", format);

    error!("starting stream");
    let mut stream = {
        let this = UserptrStream::with_buffers(&mut dev, Type::VideoCapture, 1);
        match this {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to create buffer stream {}", e);
                return Err(-1);
            }
        }
    };
    error!("stream started");

    // At this point, the stream is ready and all buffers are setup.
    // We can now read frames (represented as buffers) by iterating through
    // the stream. Once an error condition occurs, the iterator will return
    // None.

    loop {
        let _ = stream.next();
        let (buf, _) = {
            let this = stream.next();
            match this {
                Ok(t) => t,
                Err(e) => {
                    error!("Camera thread dead: {}", e);
                    return Err(-1);
                }
            }
        };
        let data = match &format.fourcc.repr {
            b"RGB3" => buf.to_vec(),
            b"MJPG" => {
                // Decode the JPEG frame to RGB
                let mut decoder = jpeg_decoder::Decoder::new(buf);
                decoder.decode().expect("failed to decode JPEG")
            }
            _ => {
                error!("invalid buffer pixelformat");
                return Err(-2);
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
