extern crate sdl2;

use super::vm::event::{EventType, Observer, VmEvent};
use sdl2::pixels::Color;
use sdl2::rect::Point;
use std::sync::mpsc::{self, Receiver, Sender};
use std::{
    thread,
    time::{Duration, Instant},
};

pub struct SdlDisplay {
    channel: Sender<VmEvent>,
}

impl SdlDisplay {
    pub fn new() -> Box<SdlDisplay> {
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();

        thread::spawn(move || SdlDisplay::handle_events(rx, ready_tx));

        ready_rx.recv().unwrap();

        Box::new(SdlDisplay { channel: tx })
    }

    fn handle_events(rx: Receiver<VmEvent>, ready_tx: Sender<()>) {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = match sdl_context.video() {
            Ok(s) => s,
            Err(s) => {
                println!("{}", s);
                return;
            }
        };
        let window = match video_subsystem
            .window("rust-sdl2 demo", 1000, 800)
            .position_centered()
            .build()
        {
            Ok(s) => s,
            Err(s) => {
                println!("{}", s);
                return;
            }
        };

        //window.set_bordered(false);
        //window.set_fullscreen(FullscreenType::True).unwrap();
        let mut canvas = match window.into_canvas().build() {
            Ok(s) => s,
            Err(s) => {
                println!("{}", s);
                return;
            }
        };
        canvas.set_scale(10f32, 10f32).unwrap();
        let mut last_display = Instant::now();

        let colors = vec![
            Color::RED,
            Color::BLUE,
            Color::GREEN,
            Color::YELLOW,
            Color::MAGENTA,
            Color::WHITE,
        ];
        let light_colors = vec![
            Color::RGBA(255, 114, 118, 255),
            Color::RGBA(164, 219, 232, 255),
            Color::RGBA(162, 228, 184, 255),
            Color::RGBA(241, 235, 156, 255),
            Color::RGBA(241, 178, 220, 255),
            Color::RGBA(240, 240, 240, 255),
        ];

        let mut sdl_event_pump = sdl_context.event_pump().unwrap();
        ready_tx.send(()).unwrap();
        loop {
            sdl_event_pump.poll_event();

            let event = rx.recv().unwrap();
            let x = (event.offset.unwrap_or(0) % 100) as i32;
            let y = (event.offset.unwrap_or(0) / 100) as i32;

            match event.event_type {
                EventType::TerminatedProgram => {
                    println!(
                        "Warrior {} terminated after {} rounds",
                        event.warrior_id, event.round
                    );
                }
                EventType::TerminatedThread => {
                    let passed_x = (event.moved_from.unwrap_or(0) % 100) as i32;
                    let passed_y = (event.moved_from.unwrap_or(0) / 100) as i32;

                    canvas.set_draw_color(colors[event.warrior_id]);
                    canvas.draw_point(Point::new(passed_x, passed_y)).unwrap();
                }
                EventType::Jump => {
                    let passed_x = (event.moved_from.unwrap_or(0) % 100) as i32;
                    let passed_y = (event.moved_from.unwrap_or(0) / 100) as i32;

                    canvas.set_draw_color(colors[event.warrior_id]);
                    canvas.draw_point(Point::new(passed_x, passed_y)).unwrap();

                    canvas.set_draw_color(light_colors[event.warrior_id]);
                    canvas.draw_point(Point::new(x, y)).unwrap();
                }
                EventType::Change => {
                    canvas.set_draw_color(colors[event.warrior_id]);
                    canvas.draw_point(Point::new(x, y)).unwrap();
                }
            }

            last_display = if last_display.elapsed() > Duration::from_millis(1000 / 24) {
                canvas.present();
                Instant::now()
            } else {
                last_display
            };
        }
    }
}

impl Observer<VmEvent> for SdlDisplay {
    fn notify(&self, event: VmEvent) {
        match event.event_type {
            EventType::TerminatedProgram => {
                println!(
                    "Warrior {} terminated after {} rounds",
                    event.warrior_id, event.round
                );
            }
            _ => {
                self.channel.send(event).unwrap();
            }
        }
    }
}
