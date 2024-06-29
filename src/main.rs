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

use std::sync::atomic::{self, Ordering};
use std::{fs, sync, thread, time};
use log::info;

use rdev::{listen, Event};
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize, Debug)]
struct GhostInformation {
    id: String,
    name: String,
    speed: String,
    features: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    version: i32,
    ghosts: Vec<GhostInformation>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    info!("加载配置文件中");
    let config: sync::Arc<sync::RwLock<Config>> = sync::Arc::new(sync::RwLock::new(
        serde_json::from_str(&fs::read_to_string("./config.json")?)?,
    ));

    let mut window = graphics::RenderWindow::new(
        (200 * SCALE, 300 * SCALE),
        "Phasutils",
        window::Style::NONE,
        &window::ContextSettings::default(),
    );
    let mut window_background = graphics::RenderWindow::new(
        (200 * SCALE, 300 * SCALE),
        "Phasutils",
        window::Style::NONE,
        &window::ContextSettings::default(),
    );
    let window_should_close = sync::Arc::new(atomic::AtomicBool::new(false));

    let h_wnd = HWND(window.system_handle() as isize);

    unsafe {
        SetWindowLongW(
            h_wnd,
            GWL_EXSTYLE,
            GetWindowLongW(h_wnd, GWL_EXSTYLE) | WS_EX_LAYERED.0 as i32,
        );
        SetLayeredWindowAttributes(h_wnd, COLORREF::default(), 0, LWA_COLORKEY)?;
    }

    window.set_position((0, 0).into());
    window.set_framerate_limit(200);

    let font = graphics::Font::from_file("./assets/font.ttf").unwrap();

    // --- 标题 --- //

    let mut text_title = graphics::Text::new("PHASUTILS by Cg1340", &font, 10 * SCALE);
    text_title.set_style(graphics::TextStyle::ITALIC);
    text_title.set_position(system::Vector2f::new(
        (10 * SCALE) as f32,
        (10 * SCALE) as f32,
    ));

    // --- 计时器 --- //

    let mut text_timer = graphics::Text::new("00:00", &font, 40 * SCALE);
    text_timer.set_fill_color(TEXT_COLOR);
    text_timer.set_position(system::Vector2f::new(
        (10 * SCALE) as f32,
        text_title.global_bounds().top + text_title.global_bounds().height,
    ));

    let tips = [
        "[1] 键开始计时",
        "[2] 键停止计时",
        "[3] 键重置",
        "[0] 键退出",
        "[Z/X] 键切换到上/下一个鬼魂特性",
    ];
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

    let stopwatch = sync::Arc::new(sync::RwLock::new(StopWatch::new()));
    let stopwatch_clone = stopwatch.clone();

    // --- 鬼魂信息 --- //
    let mut text_ghost_name = graphics::Text::new("[GHOST_NAME]", &font, 10 * SCALE);
    text_ghost_name.set_position(system::Vector2f::new(
        (10 * SCALE) as f32,
        text_tips.last().unwrap().global_bounds().top
            + text_tips.last().unwrap().global_bounds().height
            + (10 * SCALE) as f32,
    ));

    let mut text_ghost_features = graphics::Text::new("[GHOST_FEATURES]", &font, 10 * SCALE);
    text_ghost_features.set_position(system::Vector2f::new(
        (10 * SCALE) as f32,
        text_ghost_name.global_bounds().top + text_ghost_name.global_bounds().height,
    ));

    let index = sync::Arc::new(atomic::AtomicUsize::new(0));
    let ghost_information_should_update = sync::Arc::new(atomic::AtomicBool::new(true));

    let config_clone = config.clone();
    let index_clone = index.clone();
    let ghost_information_should_update_clone = ghost_information_should_update.clone();
    let window_should_close_clone = window_should_close.clone();
    thread::spawn(move || {
        let stopwatch = stopwatch_clone;

        let callback = move |event: Event| {
            if let rdev::EventType::KeyPress(key) = event.event_type {
                let mut stopwatch = stopwatch.write().unwrap();
                match key {
                    rdev::Key::Num1 => stopwatch.start(),
                    rdev::Key::Num2 => stopwatch.stop(),
                    rdev::Key::Num3 => stopwatch.reset(),
                    rdev::Key::Num0 => window_should_close_clone.store(true, Ordering::Relaxed),
                    rdev::Key::KeyZ => {
                        if index_clone.load(Ordering::Relaxed) != 0 {
                            index_clone.fetch_sub(1, Ordering::Relaxed);
                            ghost_information_should_update_clone.store(true, Ordering::Relaxed);
                        }
                    }
                    rdev::Key::KeyX => {
                        if index_clone.load(Ordering::Relaxed)
                            != config_clone.read().unwrap().ghosts.len() - 1
                        {
                            index_clone.fetch_add(1, Ordering::Relaxed);
                            ghost_information_should_update_clone.store(true, Ordering::Relaxed);
                        }
                    }
                    _ => {}
                }
            }
        };

        if let Err(error) = listen(callback) {
            println!("Error: {:?}", error);
        }
    });

    while !window_should_close.load(Ordering::Relaxed) {
        while let Some(event) = window.poll_event() {
            if event == window::Event::Closed {
                window.close()
            }
        }

        if ghost_information_should_update.load(Ordering::Relaxed) {
            let ghost_information = &config.read().unwrap().ghosts[index.load(Ordering::Relaxed)];

            text_ghost_name.set_string(&format!(
                "{}. {} ({}) {}",
                index.load(Ordering::Relaxed),
                &ghost_information.name,
                &ghost_information.id,
                &ghost_information.speed
            ));

            text_ghost_features.set_string(&ghost_information.features);
            let mut string = ghost_information.features.clone();

            let mut sum = (10 * SCALE) as f32;
            let mut byte_count = 0;
            for char in ghost_information.features.chars() {
                let tmp = font.glyph(char as u32, 10 * SCALE, false, 0f32).advance();
                sum += tmp;
                byte_count += char.len_utf8();

                println!("{:?} | advance {} | byte_count {}", char, tmp, byte_count);
                if char == '\n' {
                    sum = (10 * SCALE) as f32;
                } else if sum >= window.size().x as f32 {
                    println!("Enter {sum} | {}", window.size().x);
                    string.insert(byte_count - char.len_utf8(), '\n');
                    byte_count += '\n'.len_utf8();
                    sum = tmp + (10 * SCALE) as f32;
                }
            }

            text_ghost_features.set_string(&string);

            ghost_information_should_update.store(false, Ordering::Relaxed);
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

        window.draw(&text_title);
        window.draw(&text_timer);
        for text_tip in &text_tips {
            window.draw(text_tip);
        }

        window.draw(&text_ghost_name);
        window.draw(&text_ghost_features);

        unsafe {
            SetWindowPos(h_wnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE)?;
        }

        window.display();
    }

    window.close();
    window_background.close();

    Ok(())
}
