use std::time;

const FRAME_TIME: time::Duration = time::Duration::from_millis((1000 / 30) as u64);

pub(crate) struct FrameTimer {
    prev_tick: Option<FrameTick>,
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct FrameTick {
    pub start: time::Instant,
    pub instant: time::Instant,

    pub t: f32,
    pub dt: f32,
}

impl FrameTick {
    fn from_start() -> FrameTick {
        let now = time::Instant::now();

        FrameTick {
            start: now,
            instant: now,
            t: 0.0,
            dt: 0.0,
        }
    }

    fn from_prev(last_tick: &FrameTick) -> FrameTick {
        let start = last_tick.start;
        let instant = time::Instant::now();
        let delta = last_tick.instant.elapsed();
        let t = start.elapsed().as_secs_f32();
        let dt = delta.as_secs_f32();

        FrameTick {
            start,
            instant,
            t,
            dt,
        }
    }
}

impl FrameTimer {
    pub fn new() -> Self {
        FrameTimer { prev_tick: None }
    }

    pub fn tick(&mut self) -> FrameTick {
        if self.prev_tick.is_none() {
            self.prev_tick = Some(FrameTick::from_start());
            return self.prev_tick.unwrap();
        } else {
            self.prev_tick = Some(FrameTick::from_prev(self.prev_tick.as_ref().unwrap()))
        }

        self.prev_tick.unwrap()
    }

    pub fn wait_for_next_frame(&self) {
        if self.prev_tick.is_none() {
            return;
        }

        let delta = self.prev_tick.unwrap().instant.elapsed();
        if delta < FRAME_TIME {
            std::thread::sleep(FRAME_TIME - delta);
        }
    }
}
