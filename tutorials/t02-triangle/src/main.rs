mod v1;
mod v2;
mod v3;
mod vertex;

use clap::Parser;
use tracing::{info, Level};
use winit::{event_loop::EventLoop, window::WindowBuilder};

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value_t = String::from("v1"))]
    version: String,
}

fn main() {
    let args = Args::parse();

    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    match args.version.as_str() {
        "v1" => {
            pollster::block_on(v1::run(event_loop, window));
        }
        "v2" => {
            pollster::block_on(v2::run(event_loop, window));
        }
        "v3" => {
            pollster::block_on(v3::run(event_loop, window));
        }
        _ => {
            info!("invalid version, exit")
        }
    }
}
