//! System tray icon and context menu.

use crate::display::Resolution;
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
    SetResolution(u32, u32),
    /// 0 = highest available.
    SetRefreshRate(u32),
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
    resolutions: Vec<Resolution>,
    res_menu: Submenu,
    res_items: Vec<(CheckMenuItem, u32, u32)>,
    rate_menu: Submenu,
    rate_items: Vec<(CheckMenuItem, u32)>,
    selection: (u32, u32, u32),
}

fn aspect(w: u32, h: u32) -> &'static str {
    match (w * 3 == h * 4, w * 9 == h * 16, w * 10 == h * 16, w * 4 == h * 5) {
        (true, ..) => " (4:3)",
        (_, true, ..) => " (16:9)",
        (_, _, true, _) => " (16:10)",
        (.., true) => " (5:4)",
        _ => "",
    }
}

impl Tray {
    pub fn new(
        lang: Lang,
        resolutions: Vec<Resolution>,
        selection: (u32, u32, u32),
    ) -> Result<Self, String> {
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

        let res_menu = Submenu::new("", !resolutions.is_empty());
        let mut res_items = Vec::new();
        for r in &resolutions {
            let item = CheckMenuItem::new(
                format!("{}×{}{}", r.width, r.height, aspect(r.width, r.height)),
                true,
                false,
                None,
            );
            res_menu
                .append(&item)
                .map_err(|e| format!("tray menu: {e}"))?;
            res_items.push((item, r.width, r.height));
        }
        let rate_menu = Submenu::new("", true);

        let menu = Menu::new();
        menu.append_items(&[
            &status,
            &PredefinedMenuItem::separator(),
            &pause,
            &manual,
            &PredefinedMenuItem::separator(),
            &res_menu,
            &rate_menu,
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

        let mut tray = Self {
            icon,
            lang,
            status,
            pause,
            manual,
            lang_uk,
            lang_en,
            quit,
            state: Some(IconState::Idle),
            resolutions,
            res_menu,
            res_items,
            rate_menu,
            rate_items: Vec::new(),
            selection,
        };
        tray.set_selection(selection.0, selection.1, selection.2);
        Ok(tray)
    }

    /// Mark the chosen resolution / refresh rate and rebuild the rate list
    /// for that resolution.
    pub fn set_selection(&mut self, width: u32, height: u32, rate: u32) {
        let lang = self.lang;
        self.selection = (width, height, rate);
        for (item, w, h) in &self.res_items {
            item.set_checked(*w == width && *h == height);
        }

        for (item, _) in self.rate_items.drain(..) {
            let _ = self.rate_menu.remove(&item);
        }
        let rates = self
            .resolutions
            .iter()
            .find(|r| r.width == width && r.height == height)
            .map(|r| r.rates.clone())
            .unwrap_or_default();
        let max = CheckMenuItem::new(tr(lang, Msg::MaxRate), true, rate == 0, None);
        let _ = self.rate_menu.append(&max);
        self.rate_items.push((max, 0));
        for hz in rates {
            let item = CheckMenuItem::new(
                format!("{hz} {}", tr(lang, Msg::Hz)),
                true,
                hz == rate,
                None,
            );
            let _ = self.rate_menu.append(&item);
            self.rate_items.push((item, hz));
        }

        self.res_menu
            .set_text(format!("{}: {width}×{height}", tr(lang, Msg::Resolution)));
        let rate_label = if rate == 0 {
            tr(lang, Msg::MaxRate).to_lowercase()
        } else {
            format!("{rate} {}", tr(lang, Msg::Hz))
        };
        self.rate_menu
            .set_text(format!("{}: {rate_label}", tr(lang, Msg::RefreshRate)));
    }

    pub fn set_language(&mut self, lang: Lang) {
        self.lang = lang;
        self.quit.set_text(tr(lang, Msg::Quit));
        self.lang_uk.set_checked(lang == Lang::Uk);
        self.lang_en.set_checked(lang == Lang::En);
        let (w, h, r) = self.selection;
        self.set_selection(w, h, r);
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
            if let Some((_, w, h)) = self.res_items.iter().find(|(i, ..)| i.id() == id) {
                return Some(Action::SetResolution(*w, *h));
            }
            if let Some((_, hz)) = self.rate_items.iter().find(|(i, _)| i.id() == id) {
                return Some(Action::SetRefreshRate(*hz));
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
