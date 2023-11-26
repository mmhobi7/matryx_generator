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

const CAMERA_ON: bool = true;
const SHIFTER_START: f32 = -180.0;

fn main() {
    let client = MatrixClient::new(MatrixClientSettings {
        addrs: vec!["tcp://localhost:42024".to_string()],
    });

    #[cfg(debug_assertions)]
    let _log2 = log2::open("matryx-debug.txt")
        .size(100 * 1024 * 1024)
        .rotate(2)
        .tee(true)
        .level("trace")
        .start();

    #[cfg(not(debug_assertions))]
    let _log2 = log2::open("matryx-release.txt")
        .size(100 * 1024 * 1024)
        .rotate(2)
        .tee(false)
        .level("warn")
        .start();

    warn!("Matryx V4");
    let mut canvas_clock = Canvas::new(64, 32);
    let mut canvas_wave = Canvas::new(64, 32);
    let mut frame_timer = frame_tick::FrameTimer::new();
    let mut scene = WaveScene::new(&canvas_wave, 1.0);
    let mut clock_scene: ClockScene = ClockScene::new(&canvas_clock);
    let hists = Arc::new(AtomicU8::new(100));
    let hists_clone = hists.clone();

    if CAMERA_ON {
        let mut handle_vec = vec![]; // JoinHandles will go in here
        let handle = thread::spawn(move || camea_thread::cam_thread_loop(hists_clone));
        handle_vec.push(handle); // save the handle so we can call join on it outside of the loop
    }

    let mut shifter: f32 = SHIFTER_START;

    loop {
        let tick = frame_timer.tick();
        clock_scene.tick(&mut canvas_clock, &tick);
        debug!("camera light reading: {0}", hists.load(Ordering::Acquire));
        if hists.load(Ordering::Acquire) <= 24 {
            canvas::filter_quarter(&mut canvas_clock);
            client.send_brightness(1);
            client.send_frame(canvas_clock.pixels());
        } else {
            scene.tick(&mut canvas_wave, &tick);
            if shifter == (SHIFTER_START * (-1.0)) {
                shifter = SHIFTER_START;
            } else {
                shifter = shifter + 1.0;
            }
            canvas::filter_hue_shift(&mut canvas_wave, shifter);
            client.send_brightness(100);
            canvas::filter_rotate_right(&mut canvas_wave);
            canvas::filter_bright_background(&mut canvas_wave, &mut canvas_clock, 0.1);
            client.send_frame(canvas_wave.pixels());
        }
        frame_timer.wait_for_next_frame();
    }
}
