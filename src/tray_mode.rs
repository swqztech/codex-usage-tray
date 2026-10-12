use crate::usage;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;
use std::time::Duration;
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

enum UserEvent {
    Menu(MenuEvent),
    Refresh,
}

fn tray_icon() -> Icon {
    let mut rgba = vec![0u8; 16 * 16 * 4];
    for y in 0..16 {
        for x in 0..16 {
            let i = (y * 16 + x) * 4;
            let border = x == 0 || y == 0 || x == 15 || y == 15;
            if border {
                rgba[i] = 40;
                rgba[i + 1] = 44;
                rgba[i + 2] = 52;
                rgba[i + 3] = 255;
            } else {
                rgba[i] = 24;
                rgba[i + 1] = 27;
                rgba[i + 2] = 31;
                rgba[i + 3] = 255;
            }
            if x >= 2 && x <= 13 && y >= 10 {
                rgba[i] = 45;
                rgba[i + 1] = 212;
                rgba[i + 2] = 191;
                rgba[i + 3] = 255;
            }
        }
    }
    Icon::from_rgba(rgba, 16, 16).expect("icon")
}

fn build_menu(snap: &usage::UsageSnapshot) -> Menu {
    let menu = Menu::new();
    let _ = menu.append(&MenuItem::with_id("title", "Codex 剩余用量", true, None));
    for (idx, line) in snap.menu_lines().iter().enumerate() {
        let _ = menu.append(&MenuItem::with_id(format!("line{}", idx), line, true, None));
    }
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id("refresh", "立即刷新", true, None));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id("quit", "退出", true, None));
    menu
}

fn apply_snapshot(tray: &TrayIcon, snap: &usage::UsageSnapshot) {
    let tip = format!("Codex 用量 · {}", snap.tray_tooltip());
    let _ = tray.set_tooltip(Some(tip));
    let _ = tray.set_menu(Some(Box::new(build_menu(snap))));
}

pub fn run() {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::Menu(event));
    }));

    let refresh_proxy = event_loop.create_proxy();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(60));
            let _ = refresh_proxy.send_event(UserEvent::Refresh);
        }
    });

    let snap = Rc::new(RefCell::new(usage::load_or_placeholder()));
    let tray: Rc<RefCell<Option<TrayIcon>>> = Rc::new(RefCell::new(None));

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                let current = snap.borrow().clone();
                let icon = TrayIconBuilder::new()
                    .with_menu(Box::new(build_menu(&current)))
                    .with_tooltip(format!("Codex 用量 · {}", current.tray_tooltip()))
                    .with_icon(tray_icon())
                    .build()
                    .expect("创建托盘失败");
                icon.set_show_menu_on_left_click(true);
                *tray.borrow_mut() = Some(icon);
            }

            Event::UserEvent(UserEvent::Refresh) => {
                *snap.borrow_mut() = usage::load_or_placeholder();
                if let Some(t) = tray.borrow().as_ref() {
                    apply_snapshot(t, &*snap.borrow());
                }
            }

            Event::UserEvent(UserEvent::Menu(menu_event)) => match menu_event.id.as_ref() {
                "refresh" => {
                    *snap.borrow_mut() = usage::load_or_placeholder();
                    if let Some(t) = tray.borrow().as_ref() {
                        apply_snapshot(t, &*snap.borrow());
                    }
                }
                "quit" => {
                    tray.borrow_mut().take();
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            },

            _ => {}
        }
    });
}
