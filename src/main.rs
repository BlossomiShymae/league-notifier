#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use winit::event_loop::EventLoop;

pub mod client;
pub mod types;

fn main() {
    let event_loop = EventLoop::builder().build().unwrap();

    let mut app = client::LeagueNotifier::default();
    let _ = event_loop.run_app(&mut app);
}
