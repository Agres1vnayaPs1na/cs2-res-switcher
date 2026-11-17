//! UI translations (Ukrainian / English).

use serde::{Deserialize, Serialize};
use windows::Win32::Globalization::GetUserDefaultUILanguage;

/// Language setting as stored in `config.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Follow the Windows display language.
    Auto,
    Uk,
    En,
}

/// Resolved UI language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Uk,
    En,
}

impl Language {
    pub fn resolve(self) -> Lang {
        match self {
            Language::Uk => Lang::Uk,
            Language::En => Lang::En,
            Language::Auto => detect(),
        }
    }

    pub fn parse(code: &str) -> Option<Self> {
        match code.trim().to_ascii_lowercase().as_str() {
            "uk" | "ua" | "ukrainian" => Some(Language::Uk),
            "en" | "english" => Some(Language::En),
            "auto" => Some(Language::Auto),
            _ => None,
        }
    }
}

impl From<Lang> for Language {
    fn from(lang: Lang) -> Self {
        match lang {
            Lang::Uk => Language::Uk,
            Lang::En => Language::En,
        }
    }
}

/// Ukrainian Windows UI -> Ukrainian, anything else -> English.
fn detect() -> Lang {
    const LANG_UKRAINIAN: u16 = 0x22;
    // SAFETY: no preconditions.
    let langid = unsafe { GetUserDefaultUILanguage() };
    if langid & 0x3ff == LANG_UKRAINIAN {
        Lang::Uk
    } else {
        Lang::En
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Pause,
    Resume,
    ManualSwitch,
    ManualRestore,
    Quit,
    AlreadyRunning,
    Resolution,
    RefreshRate,
    MaxRate,
    Hz,
    Welcome,
}

pub fn tr(lang: Lang, msg: Msg) -> &'static str {
    use Msg::*;
    match (lang, msg) {
        (Lang::Uk, Pause) => "Пауза",
        (Lang::En, Pause) => "Pause",
        (Lang::Uk, Resume) => "Відновити",
        (Lang::En, Resume) => "Resume",
        (Lang::Uk, ManualSwitch) => "Перемкнути вручну",
        (Lang::En, ManualSwitch) => "Switch manually",
        (Lang::Uk, ManualRestore) => "Повернути робочий стіл",
        (Lang::En, ManualRestore) => "Restore desktop",
        (Lang::Uk, Quit) => "Вихід",
        (Lang::En, Quit) => "Exit",
        (Lang::Uk, AlreadyRunning) => {
            "cs2-res-switcher вже запущено (дивіться іконку в треї)."
        }
        (Lang::En, AlreadyRunning) => {
            "cs2-res-switcher is already running (see the tray icon)."
        }
        (Lang::Uk, Resolution) => "Роздільність",
        (Lang::En, Resolution) => "Resolution",
        (Lang::Uk, RefreshRate) => "Частота",
        (Lang::En, RefreshRate) => "Refresh rate",
        (Lang::Uk, MaxRate) => "Максимальна",
        (Lang::En, MaxRate) => "Highest",
        (Lang::Uk, Hz) => "Гц",
        (Lang::En, Hz) => "Hz",
        (Lang::Uk, Welcome) => concat!(
            "cs2-res-switcher працює у треї: іконка монітора біля годинника ",
            "(якщо її не видно, натисніть стрілку ^).\n\n",
            "Правий клік по іконці → «Роздільність» і «Частота», щоб вибрати режим для гри. ",
            "У списку лише ті режими, які підтримує ваш монітор.\n\n",
            "Коли запуститься CS2, монітор перемкнеться сам, а після гри все повернеться назад."
        ),
        (Lang::En, Welcome) => concat!(
            "cs2-res-switcher is running in the tray: the monitor icon next to the clock ",
            "(if you can't see it, click the ^ arrow).\n\n",
            "Right-click the icon → \"Resolution\" and \"Refresh rate\" to choose the game mode. ",
            "Only modes your monitor supports are listed.\n\n",
            "When CS2 starts the monitor switches automatically and goes back after the game."
        ),
    }
}

/// What the status line in the tray menu should say.
#[derive(Debug, Clone, Copy)]
pub enum Status<'a> {
    Idle,
    Active(&'a str),
    ActiveManual,
    /// Game is running but the desktop mode is kept.
    GameRunning(&'a str),
    Paused,
    PausedManual,
}

pub fn status(lang: Lang, status: Status) -> String {
    match (lang, status) {
        (Lang::Uk, Status::Idle) => "Очікування гри".into(),
        (Lang::En, Status::Idle) => "Idle".into(),
        (Lang::Uk, Status::Active(game)) => format!("Активно: {game}"),
        (Lang::En, Status::Active(game)) => format!("Active: {game}"),
        (Lang::Uk, Status::ActiveManual) => "Активно: вручну".into(),
        (Lang::En, Status::ActiveManual) => "Active: manual".into(),
        (Lang::Uk, Status::GameRunning(game)) => format!("Очікування ({game} запущено)"),
        (Lang::En, Status::GameRunning(game)) => format!("Idle ({game} running)"),
        (Lang::Uk, Status::Paused) => "Пауза".into(),
        (Lang::En, Status::Paused) => "Paused".into(),
        (Lang::Uk, Status::PausedManual) => "Пауза (ручний режим)".into(),
        (Lang::En, Status::PausedManual) => "Paused (manual mode)".into(),
    }
}
