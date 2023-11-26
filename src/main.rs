mod camea_thread;
mod canvas;
mod scenes;
mod frame_tick;

use canvas::Canvas;
use led_matrix_zmq::client::{MatrixClient, MatrixClientSettings};

use log2::*;
use std::{
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    thread::{self},
};

use scenes::{ClockScene, WaveScene};

trait Scene {
    fn tick(&mut self, _canvas: &mut Canvas, _tick: &frame_tick::FrameTick) {}
}

const LOG_SIZE: u64 = 100 * 1024 * 1024;
const LOG_ROTATE: usize = 2;

const MATRIX_ADDRS: &str = "tcp://localhost:42024";
const CANVAS_WIDTH: u32 = 64;
const CANVAS_HEIGHT: u32 = 32;

const CAMERA_ON: bool = true;
const CAMERA_LIGHT_THRESHOLD: u8 = 24;

const SHIFTER_START: f32 = -180.0;
const SHIFTER_END: f32 = SHIFTER_START * -1.0;

fn main() {
    let client = MatrixClient::new(MatrixClientSettings {
        addrs: vec![MATRIX_ADDRS.to_string()],
    });

    #[cfg(debug_assertions)]
    let _log2 = log2::open("matryx-debug.txt")
        .size(LOG_SIZE)
        .rotate(LOG_ROTATE)
        .tee(true)
        .level("trace")
        .start();

    #[cfg(not(debug_assertions))]
    let _log2 = log2::open("matryx-release.txt")
        .size(LOG_SIZE)
        .rotate(LOG_ROTATE)
        .tee(false)
        .level("warn")
        .start();

    warn!("Matryx V4");
    let mut canvas_clock = Canvas::new(CANVAS_WIDTH, CANVAS_HEIGHT);
    let mut canvas_wave = Canvas::new(CANVAS_WIDTH, CANVAS_HEIGHT);
    let mut frame_timer = frame_tick::FrameTimer::new();
    let mut scene = WaveScene::new(&canvas_wave, 1.0);
    let mut clock_scene: ClockScene = ClockScene::new(&canvas_clock);
    let camera_light_reading = Arc::new(AtomicU8::new(100));
    let camera_light_reading_clone = camera_light_reading.clone();

    if CAMERA_ON {
        let mut handle_vec = vec![]; // JoinHandles will go in here
        let handle = thread::spawn(move || camea_thread::cam_thread_loop(camera_light_reading_clone));
        handle_vec.push(handle); // save the handle so we can call join on it outside of the loop
    }

    let mut shifter: f32 = SHIFTER_START;

    loop {
        let tick = frame_timer.tick();
        clock_scene.tick(&mut canvas_clock, &tick);
        let light_reading = camera_light_reading.load(Ordering::Acquire);
        
        #[cfg(not(debug_assertions))]
        debug!("camera light reading: {0}", light_reading);

        if light_reading <= CAMERA_LIGHT_THRESHOLD {
            canvas::filter_quarter(&mut canvas_clock);
            client.send_brightness(1);
            client.send_frame(canvas_clock.pixels());
        } else {
            scene.tick(&mut canvas_wave, &tick);
            shifter = if shifter == SHIFTER_END {
                SHIFTER_START
            } else {
                shifter + 1.0
            };
            canvas::filter_hue_shift(&mut canvas_wave, shifter);
            client.send_brightness(100);
            canvas::filter_rotate_right(&mut canvas_wave);
            canvas::filter_bright_background(&mut canvas_wave, &mut canvas_clock, 0.1);
            client.send_frame(canvas_wave.pixels());
        }
        frame_timer.wait_for_next_frame();
    }
}