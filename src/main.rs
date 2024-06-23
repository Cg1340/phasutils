// phasutils: 为Steam游戏恐鬼症设计的实用工具
// Copyright (C) 2024  Chen Siyuan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, RwLock};
use std::{thread, time};

use rdev::{listen, Event};
use sfml::graphics::{RenderTarget, Transformable};
use sfml::{graphics, system, window};
use windows::Win32::Foundation::{COLORREF, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW, SetWindowPos, GWL_EXSTYLE,
    HWND_TOPMOST, LWA_COLORKEY, SWP_NOMOVE, SWP_NOSIZE, WS_EX_LAYERED,
};

const SCALE: u32 = 3;
const TEXT_COLOR: graphics::Color = graphics::Color::rgb(0x66, 0xcc, 0xff);
const TEXT_COLOR_HIGHLIGHT: graphics::Color = graphics::Color::rgb(0xff, 0xd7, 0x00);

struct StopWatch {
    elapsed: time::Duration,
    start: bool,
    instant: time::Instant,
}

impl StopWatch {
    fn new() -> StopWatch {
        StopWatch {
            elapsed: time::Duration::new(0, 0),
            start: false,
            instant: time::Instant::now(),
        }
    }

    fn start(&mut self) {
        if !self.start {
            self.start = true;
            self.instant = time::Instant::now();
        }
    }

    fn stop(&mut self) {
        if self.start {
            self.start = false;
            self.elapsed += self.instant.elapsed();
        }
    }

    fn reset(&mut self) {
        if !self.start {
            self.elapsed = time::Duration::new(0, 0);
            self.instant = time::Instant::now();
        }
    }

    fn elapsed(&self) -> time::Duration {
        self.elapsed
            + if self.start {
                self.instant.elapsed()
            } else {
                time::Duration::new(0, 0)
            }
    }
}

fn main() {
    let mut window = graphics::RenderWindow::new(
        (200 * SCALE, 300 * SCALE),
        "Phasutils",
        window::Style::NONE,
        &window::ContextSettings::default(),
    );

    #[cfg(windows)]
    let h_wnd = HWND(window.system_handle() as isize);

    #[cfg(windows)]
    unsafe {
        SetWindowLongW(
            h_wnd,
            GWL_EXSTYLE,
            GetWindowLongW(h_wnd, GWL_EXSTYLE) | WS_EX_LAYERED.0 as i32,
        );
        SetLayeredWindowAttributes(h_wnd, COLORREF::default(), 0, LWA_COLORKEY).unwrap();
    }

    window.set_position((0, 0).into());
    window.set_framerate_limit(200);

    let font = graphics::Font::from_file("./assets/font.ttf").unwrap();
    let mut text_timer = graphics::Text::new("00:00", &font, 40 * SCALE);
    text_timer.set_fill_color(TEXT_COLOR);
    text_timer.set_position(system::Vector2f::new(
        (10 * SCALE) as f32,
        (10 * SCALE) as f32,
    ));

    let tips = ["[1] 键开始计时", "[2] 键停止计时", "[3] 键重置"];
    let mut text_tips: Vec<graphics::Text> = vec![];
    for (idx, tip) in tips.iter().enumerate() {
        let mut text = graphics::Text::new(*tip, &font, 10 * SCALE);
        text.set_fill_color(TEXT_COLOR);
        if idx == 0 {
            text.set_position(system::Vector2f::new(
                (10 * SCALE) as f32,
                text_timer.global_bounds().top
                    + text_timer.global_bounds().height
                    + (10 * SCALE) as f32,
            ));
        } else {
            text.set_position(system::Vector2f::new(
                (10 * SCALE) as f32,
                text_tips[idx - 1].global_bounds().top + text_tips[idx - 1].global_bounds().height,
            ));
        }

        text_tips.push(text);
    }

    let mut text_fps_count = graphics::Text::new("", &font, 6 * SCALE);
    let mut instant_fps_count;

    let stopwatch = Arc::new(RwLock::new(StopWatch::new()));

    let stopwatch_clone = stopwatch.clone();

    thread::spawn(move || {
        let stopwatch = stopwatch_clone;

        let callback = move |event: Event| {
            if let rdev::EventType::KeyPress(key) = event.event_type {
                let mut stopwatch = stopwatch.write().unwrap();
                match key {
                    rdev::Key::Num1 => stopwatch.start(),
                    rdev::Key::Num2 => stopwatch.stop(),
                    rdev::Key::Num3 => stopwatch.reset(),
                    _ => {}
                }
            }
        };

        if let Err(error) = listen(callback) {
            println!("Error: {:?}", error);
        }
    });

    while window.is_open() {
        instant_fps_count = time::Instant::now();

        while let Some(event) = window.poll_event() {
            if event == window::Event::Closed {
                window.close()
            }
        }

        {
            let stopwatch = stopwatch.read().unwrap();
            text_timer.set_string(&format!(
                "{:02}:{:02}",
                stopwatch.elapsed().as_secs() / 60,
                stopwatch.elapsed().as_secs() % 60
            ));

            if stopwatch.start {
                text_tips[0].set_fill_color(TEXT_COLOR_HIGHLIGHT);
            } else {
                text_tips[0].set_fill_color(TEXT_COLOR);
            }
        }

        window.clear(graphics::Color::BLACK);

        window.draw(&text_timer);
        for text_tip in &text_tips {
            window.draw(text_tip);
        }
        window.draw(&text_fps_count);

        #[cfg(windows)]
        unsafe {
            SetWindowPos(h_wnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE).unwrap();
        }

        window.display();
        text_fps_count.set_string(&(1f32 / instant_fps_count.elapsed().as_secs_f32()).to_string());
    }
}
