use crate::{frame_tick::FrameTick, Canvas, Scene};
use chrono::Local;
use embedded_graphics::{geometry::Point, pixelcolor::Rgb888, prelude::*, text::Text};
use u8g2_fonts::{fonts, U8g2TextStyle};

extern crate chrono;

pub struct ClockScene();

impl ClockScene {
    pub fn new(_canvas: &Canvas) -> Self {
        ClockScene()
    }
}

impl Scene for ClockScene {
    fn tick(&mut self, canvas: &mut Canvas, _tick: &FrameTick) {
        let date = Local::now();
        canvas.clear();

        let times = date.format("%I:%M").to_string();
        let character_style =
            U8g2TextStyle::new(fonts::u8g2_font_helvB14_tn, Rgb888::new(255, 255, 255));

        Text::new(&times, Point::new(9, 22), character_style)
            .draw(canvas)
            .unwrap();
    }
}
