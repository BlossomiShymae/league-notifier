use std::{
    collections::HashMap,
    io::{self, Write},
    process::exit,
};

use crossbeam_channel::Receiver;

use irelia::{rest::LcuClient, RequestClient};
use notify_rust::Notification;
use tempfile::NamedTempFile;
use tray_icon::{
    menu::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, SubmenuBuilder},
    TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use winit::{
    application::ApplicationHandler,
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};

use crate::types::FriendResource;

pub struct LeagueNotifier {
    path: String,
    menu_channel: Receiver<MenuEvent>,
    tray_channel: Receiver<TrayIconEvent>,
    tray_icon: Option<TrayIcon>,
    quit_item: Option<MenuItem>,
}

impl LeagueNotifier {
    pub fn run(&mut self) {
        // Run period interval that fetches and compares friend list
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();

            rt.block_on(process_friends());
        });
    }
}

async fn process_friends() {
    let mut friend_map: HashMap<String, FriendResource> = HashMap::new();

    loop {
        let client = RequestClient::new();

        if let Ok(lcu_client) = LcuClient::new(true) {
            let res = lcu_client
                .get::<Vec<crate::types::FriendResource>>("/lol-chat/v1/friends", &client)
                .await;

            if let Ok(maybe_friends) = res {
                if let Some(friends) = maybe_friends {
                    compare_friend_availability(friends, &mut friend_map).await;
                }
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn compare_friend_availability(
    friends: Vec<FriendResource>,
    friend_map: &mut HashMap<String, FriendResource>,
) {
    for friend in friends {
        if (!friend_map.contains_key(&friend.puuid)) {
            friend_map.insert(friend.puuid.clone(), friend);
            continue;
        }

        // We already checked the key so unwrap
        let friend_span = friend_map.get(&friend.puuid).unwrap();

        // Friend status changed
        if (friend_span.availability.ne(&friend.availability)) {
            // If friend is not online using the League client
            if (friend.product.ne("league_of_legends")) {
                return;
            }

            match friend_span.availability.as_str() {
                "mobile" | "offline" => {
                    // Friend is online
                    match friend.availability.as_str() {
                        "chat" | "dnd" | "away" => {
                            let riot_id =
                                format!("{}#{} is now online!", friend.game_name, friend.game_tag);

                            // Get profile icon
                            let icon_url = format!("https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/profile-icons/{}.jpg", friend.icon);
                            let mut res = reqwest::get(icon_url).await.unwrap();
                            let mut file = NamedTempFile::new().unwrap();

                            while let Some(chunk) = res.chunk().await.unwrap() {
                                file.write_all(&chunk);
                            }

                            let icon_path = file.into_temp_path();

                            // Send notification
                            let _ = Notification::new()
                                .summary("League Notifier")
                                .appname("League Notifier")
                                // https://learn.microsoft.com/en-us/uwp/schemas/tiles/toastschema/element-audio?redirectedfrom=MSDN
                                // ms-winsoundevent:Notification.IM
                                .sound_name("IM")
                                .icon(icon_path.to_str().unwrap())
                                .body(riot_id.as_str())
                                .show();
                        }
                        _ => {}
                    }
                }
                _ => {}
            }

            // Update local friend map
            friend_map.insert(friend.puuid.clone(), friend);
        }
    }
}

impl Default for LeagueNotifier {
    fn default() -> Self {
        Self {
            path: String::from(concat!(env!("CARGO_MANIFEST_DIR"), "/icon.png")),
            menu_channel: MenuEvent::receiver().clone(),
            tray_channel: TrayIconEvent::receiver().clone(),
            tray_icon: None,
            quit_item: None,
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

                let tray_menu = Menu::new();
                self.quit_item = Some(MenuItem::new("Quit", true, None));

                tray_menu.append(self.quit_item.as_ref().unwrap());

                let quit_item = MenuItem::new("Quit", true, None);

                // We create the icon once the event loop is actually running
                // to prevent issues like https://github.com/tauri-apps/tray-icon/issues/90
                self.tray_icon = Some(
                    TrayIconBuilder::new()
                        .with_menu(Box::new(tray_menu))
                        .with_tooltip("League Notifier")
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

                let icon_url = format!("https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/profile-icons/4569.jpg");
                let mut res = reqwest::blocking::get(icon_url).unwrap();
                let mut file = NamedTempFile::new().unwrap();
                let mut buf: Vec<u8> = vec![];
                res.copy_to(&mut buf).unwrap();
                file.write_all(&mut buf.as_slice()).unwrap();

                let icon_path = file.into_temp_path();
                println!("{}", icon_path.display());

                // Send notification
                let _ = Notification::new()
                    .summary("League Notifier")
                    .appname("League Notifier")
                    // https://learn.microsoft.com/en-us/uwp/schemas/tiles/toastschema/element-audio?redirectedfrom=MSDN
                    // ms-winsoundevent:Notification.IM
                    .sound_name("IM")
                    .icon(icon_path.to_str().unwrap())
                    .body("Test.")
                    .show();

                self.run();
            }
            _ => {}
        };

        if let Ok(event) = self.menu_channel.try_recv() {
            if let Some(quit_item) = &self.quit_item {
                if event.id == quit_item.id() {
                    // Exit program
                    event_loop.exit();
                }
            }
        }

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
