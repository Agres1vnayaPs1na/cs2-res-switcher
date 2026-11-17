//! System tray icon and context menu.

use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    Idle,
    Active,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    TogglePause,
    ManualSwitch,
    Quit,
}

pub struct Tray {
    icon: TrayIcon,
    status: MenuItem,
    pause: MenuItem,
    manual: MenuItem,
    quit: MenuItem,
    state: Option<IconState>,
}

impl Tray {
    pub fn new() -> Result<Self, String> {
        let status = MenuItem::new("Idle", false, None);
        let pause = MenuItem::new("Пауза", true, None);
        let manual = MenuItem::new("Перемкнути вручну", true, None);
        let quit = MenuItem::new("Вихід", true, None);

        let menu = Menu::new();
        menu.append_items(&[
            &status,
            &PredefinedMenuItem::separator(),
            &pause,
            &manual,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .map_err(|e| format!("tray menu: {e}"))?;

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("cs2-res-switcher: Idle")
            .with_icon(make_icon(IconState::Idle)?)
            .build()
            .map_err(|e| format!("tray icon: {e}"))?;

        Ok(Self {
            icon,
            status,
            pause,
            manual,
            quit,
            state: Some(IconState::Idle),
        })
    }

    /// Update status text, tooltip, pause label and icon colour.
    pub fn update(&mut self, status: &str, state: IconState, paused: bool, switched: bool) {
        self.status.set_text(status);
        self.pause.set_text(if paused { "Відновити" } else { "Пауза" });
        self.manual.set_text(if switched {
            "Повернути робочий стіл"
        } else {
            "Перемкнути вручну"
        });
        let _ = self
            .icon
            .set_tooltip(Some(format!("cs2-res-switcher: {status}")));

        if self.state != Some(state) {
            if let Ok(icon) = make_icon(state) {
                let _ = self.icon.set_icon(Some(icon));
                self.state = Some(state);
            }
        }
    }

    /// Returns the next pending menu action, if any (non-blocking).
    pub fn poll_action(&self) -> Option<Action> {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            let id = event.id();
            if id == self.pause.id() {
                return Some(Action::TogglePause);
            }
            if id == self.manual.id() {
                return Some(Action::ManualSwitch);
            }
            if id == self.quit.id() {
                return Some(Action::Quit);
            }
        }
        None
    }
}

/// Draws a 32x32 "monitor" icon in code: coloured screen, dark frame, stand.
fn make_icon(state: IconState) -> Result<Icon, String> {
    const SIZE: u32 = 32;
    let screen: [u8; 4] = match state {
        IconState::Idle => [120, 130, 145, 255],
        IconState::Active => [40, 200, 90, 255],
        IconState::Paused => [235, 170, 40, 255],
    };
    let frame = [30, 30, 35, 255];
    let clear = [0, 0, 0, 0];

    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let px = if (2..=29).contains(&x) && (4..=23).contains(&y) {
                if (4..=27).contains(&x) && (6..=21).contains(&y) {
                    screen
                } else {
                    frame
                }
            } else if (14..=17).contains(&x) && (24..=26).contains(&y)
                || (9..=22).contains(&x) && (27..=28).contains(&y)
            {
                frame
            } else {
                clear
            };
            rgba.extend_from_slice(&px);
        }
    }
    Icon::from_rgba(rgba, SIZE, SIZE).map_err(|e| format!("icon: {e}"))
}
