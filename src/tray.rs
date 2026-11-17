//! System tray icon and context menu.

use crate::i18n::{tr, Lang, Msg};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
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
    SetLanguage(Lang),
    Quit,
}

pub struct Tray {
    icon: TrayIcon,
    lang: Lang,
    status: MenuItem,
    pause: MenuItem,
    manual: MenuItem,
    lang_uk: CheckMenuItem,
    lang_en: CheckMenuItem,
    quit: MenuItem,
    state: Option<IconState>,
}

impl Tray {
    pub fn new(lang: Lang) -> Result<Self, String> {
        let status = MenuItem::new("", false, None);
        let pause = MenuItem::new(tr(lang, Msg::Pause), true, None);
        let manual = MenuItem::new(tr(lang, Msg::ManualSwitch), true, None);
        let quit = MenuItem::new(tr(lang, Msg::Quit), true, None);

        // Language names are shown in their own language so anyone can find them.
        let lang_uk = CheckMenuItem::new("Українська", true, lang == Lang::Uk, None);
        let lang_en = CheckMenuItem::new("English", true, lang == Lang::En, None);
        let lang_menu = Submenu::new("Мова / Language", true);
        lang_menu
            .append_items(&[&lang_uk, &lang_en])
            .map_err(|e| format!("tray menu: {e}"))?;

        let menu = Menu::new();
        menu.append_items(&[
            &status,
            &PredefinedMenuItem::separator(),
            &pause,
            &manual,
            &lang_menu,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .map_err(|e| format!("tray menu: {e}"))?;

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("cs2-res-switcher")
            .with_icon(make_icon(IconState::Idle)?)
            .build()
            .map_err(|e| format!("tray icon: {e}"))?;

        Ok(Self {
            icon,
            lang,
            status,
            pause,
            manual,
            lang_uk,
            lang_en,
            quit,
            state: Some(IconState::Idle),
        })
    }

    pub fn set_language(&mut self, lang: Lang) {
        self.lang = lang;
        self.quit.set_text(tr(lang, Msg::Quit));
        self.lang_uk.set_checked(lang == Lang::Uk);
        self.lang_en.set_checked(lang == Lang::En);
    }

    /// Update status text, tooltip, menu labels and icon colour.
    pub fn update(&mut self, status: &str, state: IconState, paused: bool, switched: bool) {
        let lang = self.lang;
        self.status.set_text(status);
        self.pause
            .set_text(tr(lang, if paused { Msg::Resume } else { Msg::Pause }));
        self.manual.set_text(tr(
            lang,
            if switched {
                Msg::ManualRestore
            } else {
                Msg::ManualSwitch
            },
        ));
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
            if id == self.lang_uk.id() {
                return Some(Action::SetLanguage(Lang::Uk));
            }
            if id == self.lang_en.id() {
                return Some(Action::SetLanguage(Lang::En));
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
