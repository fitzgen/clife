extern crate crossbeam;
extern crate docopt;
#[macro_use]
extern crate glium;
extern crate rand;
extern crate rustc_serialize;

use docopt::Docopt;

use std::env;
use std::path;
use std::sync;
use std::sync::mpsc;
use std::thread;

pub mod draw;
pub mod error;
pub mod world;

#[cfg_attr(rustfmt, rustfmt_skip)]
pub const USAGE: &'static str = "
Usage:
  clife [-r <rate>] <file>
  clife [-r <rate>] <width> <height>
  clife (-h | --help)

Options:
  -h --help              Print this message and exit.
  -r --rate <rate>       The number of milliseconds to wait between frames.
                         [default: 16]

If <file> is supplied, it is the path to a text file containing the initial
world. The '_' character denotes a cell that is dead and '#' a cell that is
alive. For example, this would create a 5x5 world containing a \"blinker\":

_____
__#__
__#__
__#__
_____

Alternatively, if <width> and <height> are supplied, randomly generate a start
world of the given size.
";

#[derive(RustcDecodable)]
struct Args {
    arg_file: Option<String>,
    arg_width: Option<i32>,
    arg_height: Option<i32>,
    flag_help: bool,
    flag_rate: u64,
}

pub fn main() {
    let args: Args = Docopt::new(USAGE)
                         .and_then(|d| d.argv(env::args().into_iter()).decode())
                         .unwrap_or_else(|e| e.exit());
    if args.flag_help {
        return;
    }

    let current_world =
        sync::Arc::new(sync::Mutex::new(if let Some(ref file_path) = args.arg_file {
            world::World::from_file(&path::Path::new(file_path)).expect("Could not create world!")
        } else {
            world::World::new_random(args.arg_width.unwrap(), args.arg_height.unwrap())
        }));

    let mut next_world = {
        let w = current_world.lock().unwrap();
        let mut next = world::World::new_empty(w.width(), w.height());
        next.become_next_step(&*w);
        sync::Arc::new(sync::Mutex::new(next))
    };

    let mut next_next_world = {
        let w = next_world.lock().unwrap();
        let next = world::World::new_empty(w.width(), w.height());
        sync::Arc::new(sync::Mutex::new(next))
    };

    let (send_ready_to_draw, on_ready_to_draw) = mpsc::channel();
    let (send_done_drawing, on_done_drawing) = mpsc::channel();

    send_ready_to_draw.send(current_world).expect("Could not send initial world to draw!");

    thread::spawn(move || {
        for world in on_done_drawing {
            {
                let mut w = next_next_world.lock().unwrap();
                let next = next_world.lock().unwrap();
                w.become_next_step(&*next);
            }

            send_ready_to_draw.send(next_world).expect("Could not send world to draw!");

            next_world = next_next_world;
            next_next_world = world;
        }
    });

    let rate = args.flag_rate;
    draw::draw_loop(rate, on_ready_to_draw, send_done_drawing);
}
