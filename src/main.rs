#![allow(unused)]

use winit::event_loop::EventLoop;

pub mod client;
pub mod types;

fn main() {
    let event_loop = EventLoop::builder().build().unwrap();

    let mut app = client::LeagueNotifier::default();
    event_loop.run_app(&mut app);
}
