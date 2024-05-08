#![allow(unused)]

use crossbeam_channel::Receiver;

use notify_rust::Notification;
use tray_icon::{
    menu::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use winit::{
    application::ApplicationHandler,
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};

fn main() {
    let event_loop = EventLoop::builder().build().unwrap();

    let mut app = LeagueNotifier::default();
    event_loop.run_app(&mut app);
}

struct LeagueNotifier {
    path: String,
    menu_channel: Receiver<MenuEvent>,
    tray_channel: Receiver<TrayIconEvent>,
    tray_icon: Option<TrayIcon>,
}

impl Default for LeagueNotifier {
    fn default() -> Self {
        Self {
            path: String::from(concat!(env!("CARGO_MANIFEST_DIR"), "/icon.png")),
            menu_channel: MenuEvent::receiver().clone(),
            tray_channel: TrayIconEvent::receiver().clone(),
            tray_icon: None,
        }
    }
}

impl ApplicationHandler for LeagueNotifier {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        // We add delay of 16 ms (60fps) to event_loop to reduce cpu load.
        // This can be removed to allow ControlFlow::Poll to poll on each cpu cycle
        // Alternatively, you can set ControlFlow::Wait or use TrayIconEvent::set_event_handler,
        // see https://github.com/tauri-apps/tray-icon/issues/83#issuecomment-1697773065
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(16),
        ));

        match cause {
            winit::event::StartCause::Init => {
                let icon = load_icon(std::path::Path::new(self.path.as_str()));

                // We create the icon once the event loop is actually running
                // to prevent issues like https://github.com/tauri-apps/tray-icon/issues/90
                self.tray_icon = Some(
                    TrayIconBuilder::new()
                        .with_menu(Box::new(Menu::new()))
                        .with_tooltip("winit - awesome windowing lib")
                        .with_icon(icon)
                        .with_title("x")
                        .build()
                        .unwrap(),
                );
                // We have to request a redraw here to have the icon actually show up.
                // Winit only exposes a redraw method on the Window so we use core-foundation directly.
                #[cfg(target_os = "macos")]
                unsafe {
                    use core_foundation::runloop::{CFRunLoopGetMain, CFRunLoopWakeUp};

                    let rl = CFRunLoopGetMain();
                    CFRunLoopWakeUp(rl);
                }

                let _ = Notification::new()
                    .summary("Firefox News")
                    .body("This will almost look like a real firefox notification.")
                    .icon("firefox")
                    .appname("League Notifier")
                    .sound_name("Default")
                    .show();
            }
            _ => {}
        };

        if let Ok(event) = self.tray_channel.try_recv() {
            println!("{event:?}");
        }
    }
}

fn load_icon(path: &std::path::Path) -> tray_icon::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}
