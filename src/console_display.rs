use super::vm::event::{EventType, Observer, VmEvent};
use std::cell::RefCell;
use std::io::{stdout, Stdout, Write};

pub use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue,
    style::{style, Color, PrintStyledContent, SetForegroundColor, Stylize},
    terminal::{self, ClearType},
    Command, ExecutableCommand, QueueableCommand, Result,
};

pub struct ConsoleDisplay {
    stdout: RefCell<Stdout>,
    colors: Vec<Color>,
}

impl ConsoleDisplay {
    pub fn new() -> Box<ConsoleDisplay> {
        Box::new(ConsoleDisplay {
            stdout: RefCell::new(stdout()),
            colors: vec![
                Color::Red,
                Color::Blue,
                Color::Grey,
                Color::Yellow,
                Color::Green,
            ],
        })
    }
}

impl Observer<VmEvent> for ConsoleDisplay {
    fn notify(&self, event: VmEvent) {
        let x = (event.offset.unwrap_or(0) % 160) as u16;
        let y = (event.offset.unwrap_or(0) / 160) as u16;
        let mut console = self.stdout.borrow_mut();

        match event.event_type {
            EventType::TerminatedProgram => {
                let styled = style(format!("Warrior {} terminated", event.warrior_id))
                    .with(self.colors[event.warrior_id]);
                console.queue(cursor::MoveTo(0, 81));
                console.queue(PrintStyledContent(styled));
            }
            EventType::TerminatedThread => {}
            EventType::Jump => {
                let passed_x = (event.moved_from.unwrap_or(0) % 160) as u16;
                let passed_y = (event.moved_from.unwrap_or(0) / 160) as u16;

                let passed = style(".").with(self.colors[event.warrior_id]);
                let head = style("*").with(self.colors[event.warrior_id]);

                console.queue(cursor::MoveTo(passed_x, passed_y));
                console.queue(PrintStyledContent(passed));
                console.queue(cursor::MoveTo(x, y));
                console.queue(PrintStyledContent(head));
            }
            EventType::Change => {
                let styled = style(".").with(self.colors[event.warrior_id]);

                console.queue(cursor::MoveTo(x, y));
                console.queue(PrintStyledContent(styled));
            }
        }

        if event.round % 1000 == 0 {
            let styled = style(format!("Change #{}", event.round)).with(Color::White);

            console.queue(cursor::MoveTo(0, 82));
            console.queue(PrintStyledContent(styled));

            console.flush();
        }
    }
}
