mod vm;

use std::sync::mpsc::channel;
use vm::{
    event::Observable,
    parser::parse,
    vms::{Vm, WarriorDefinition},
};
mod console_display;
mod sdl_display;
use clap::Parser;
use sdl_display::SdlDisplay;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

#[derive(Parser)]
struct CliArgs {
    path: PathBuf,
}

fn read_warrior<const CORE_SIZE: usize>(path: &str) -> Result<WarriorDefinition<CORE_SIZE>, ()> {
    let name = Path::new(path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let body = fs::read_to_string(path).expect(&format!("Can not open file {}", path));
    let instructions = parse(body).expect(&format!("Can not parse instructions in file {}", path));

    Ok(WarriorDefinition::new(name, instructions))
}

fn main() {
    let args = CliArgs::parse();
    let path = fs::read_dir(args.path).unwrap();

    let warriors: Vec<WarriorDefinition<8000>> = path
        .map(|f| f.unwrap().path())
        .map(|path| path.to_str().unwrap().to_string())
        .filter(|path| path.ends_with(".war"))
        .filter_map(|path| read_warrior(&path).ok())
        .collect();

    let (timer_tx, timer_rx) = channel();
    thread::spawn(move || loop {
        timer_tx.send(()).unwrap();
        thread::sleep(Duration::from_millis(25));
    });

    //let console_display = ConsoleDisplay::new();
    let sdl_display = SdlDisplay::new();

    let mut vm = Vm::<8000, 32>::new(warriors).unwrap();
    vm.register(sdl_display);
    'game_loop: loop {
        timer_rx.recv().unwrap();
        match vm.play(64) {
            None => {
                println!("Played {} rounds", vm.round);
            }
            Some(p) => {
                println!("Game ended! Player {} won!", p.name);
                break 'game_loop;
            }
        }
    }
}
