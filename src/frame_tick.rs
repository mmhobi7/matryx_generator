use std::time;

const FRAME_TIME: time::Duration = time::Duration::from_millis((1000.0 / 30.0) as u64);

pub struct FrameTimer {
    prev_tick: Option<FrameTick>,
}

#[derive(Copy, Clone, Debug)]
pub struct FrameTick {
    pub start: time::Instant,
    pub instant: time::Instant,
    pub t: f32,
    pub dt: f32,
}

impl FrameTick {
    fn new(start: time::Instant, instant: time::Instant, t: f32,dt: f32) -> FrameTick {
        FrameTick {
            start,
            instant,
            t,
            dt,
        }
    }

    fn from_start() -> FrameTick {
        let now = time::Instant::now();
        FrameTick::new(now, now, 0.0, 0.0)
    }

    fn from_prev(last_tick: &FrameTick) -> FrameTick {
        let start = last_tick.start;
        let instant = time::Instant::now();
        let t = start.elapsed().as_secs_f32();
        let dt = last_tick.instant.elapsed().as_secs_f32();
        FrameTick::new(start, instant, t, dt)
    }
}

impl FrameTimer {
    pub fn new() -> Self {
        FrameTimer { prev_tick: None }
    }

    pub fn tick(&mut self) -> FrameTick {
        self.prev_tick = Some(match self.prev_tick {
            None => FrameTick::from_start(),
            Some(prev_tick) => FrameTick::from_prev(&prev_tick),
        });

        self.prev_tick.unwrap()
    }

    pub fn wait_for_next_frame(&self) {
        if let Some(prev_tick) = self.prev_tick {
            let delta = prev_tick.instant.elapsed();
            if delta < FRAME_TIME {
                std::thread::sleep(FRAME_TIME - delta);
            }
        }
    }
}