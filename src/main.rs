//! cs2-res-switcher: switches the PRIMARY monitor to a game resolution while
//! the target process runs and restores the desktop mode afterwards.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod display;
mod logger;
mod tray;

use config::Config;
use display::Switcher;
use std::ffi::OsString;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use tray::{Action, IconState, Tray};
use windows::core::{w, BOOL};
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, LPARAM, WPARAM};
use windows::Win32::System::Console::SetConsoleCtrlHandler;
use windows::Win32::System::Threading::{CreateMutexW, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, KillTimer, MessageBoxW, PostQuitMessage, PostThreadMessageW,
    SetTimer, TranslateMessage, MB_ICONINFORMATION, MB_OK, MSG, WM_APP, WM_TIMER,
};

/// Message-loop tick; keeps the tray menu responsive. Process polling uses
/// `poll_interval_ms` on top of this.
const TICK_MS: u32 = 100;
const WM_APP_QUIT: u32 = WM_APP + 1;

static MAIN_THREAD_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Desktop mode, waiting for the game.
    Idle,
    /// Switched because the game is running; restored when it exits.
    Auto,
    /// Switched by the user from the tray menu.
    Manual,
    /// Game is running but we are not switched (user restored manually or
    /// the switch failed). Waits for the game to exit before re-arming.
    Suppressed,
}

struct App {
    cfg: Config,
    target: OsString,
    sys: System,
    switcher: Switcher,
    tray: Tray,
    mode: Mode,
    paused: bool,
}

impl App {
    fn game_running(&mut self) -> bool {
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing(),
        );
        self.sys
            .processes()
            .values()
            .any(|p| p.name().eq_ignore_ascii_case(&self.target))
    }

    fn switch_to_game(&mut self) -> bool {
        let Config {
            width,
            height,
            refresh_rate,
            scaling,
            ..
        } = self.cfg;
        match self.switcher.apply(width, height, refresh_rate, scaling) {
            Ok(()) => true,
            Err(e) => {
                log_error!("Switch failed: {e}");
                false
            }
        }
    }

    fn restore(&mut self) {
        if let Err(e) = self.switcher.restore() {
            log_error!("{e}");
        }
    }

    fn tick(&mut self) {
        if self.paused {
            return;
        }
        let running = self.game_running();
        let next = match (self.mode, running) {
            (Mode::Idle, true) => {
                log_info!("{} started", self.cfg.process_name);
                if self.switch_to_game() {
                    Mode::Auto
                } else {
                    Mode::Suppressed
                }
            }
            (Mode::Auto, false) => {
                log_info!("{} exited", self.cfg.process_name);
                self.restore();
                Mode::Idle
            }
            (Mode::Manual, true) => Mode::Auto,
            (Mode::Suppressed, false) => Mode::Idle,
            (mode, _) => mode,
        };
        self.set_mode(next);
    }

    fn handle(&mut self, action: Action) -> bool {
        match action {
            Action::TogglePause => {
                self.paused = !self.paused;
                if self.paused {
                    self.restore();
                    log_info!("Paused");
                    self.set_mode(Mode::Idle);
                } else {
                    log_info!("Resumed");
                    self.tick();
                }
            }
            Action::ManualSwitch => {
                if self.switcher.is_switched() {
                    log_info!("Manual restore");
                    self.restore();
                    let running = !self.paused && self.game_running();
                    self.set_mode(if running { Mode::Suppressed } else { Mode::Idle });
                } else {
                    log_info!("Manual switch");
                    if self.switch_to_game() {
                        self.set_mode(Mode::Manual);
                    }
                }
            }
            Action::Quit => {
                log_info!("Exit requested");
                self.restore();
                return false;
            }
        }
        true
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.refresh_tray();
    }

    fn refresh_tray(&mut self) {
        let switched = self.switcher.is_switched();
        let status = match (self.paused, self.mode) {
            (true, _) if switched => "Paused (manual mode)".to_string(),
            (true, _) => "Paused".to_string(),
            (false, Mode::Auto) => format!("Active: {}", self.cfg.process_name),
            (false, Mode::Manual) => "Active: manual".to_string(),
            (false, Mode::Suppressed) => format!("Idle ({} running)", self.cfg.process_name),
            (false, Mode::Idle) => "Idle".to_string(),
        };
        let state = if switched {
            IconState::Active
        } else if self.paused {
            IconState::Paused
        } else {
            IconState::Idle
        };
        self.tray.update(&status, state, self.paused, switched);
    }
}

/// Console Ctrl+C / close handler (debug builds have a console).
unsafe extern "system" fn console_handler(_ctrl_type: u32) -> BOOL {
    let tid = MAIN_THREAD_ID.load(Ordering::SeqCst);
    // SAFETY: posting a message to our own UI thread.
    let _ = unsafe { PostThreadMessageW(tid, WM_APP_QUIT, WPARAM(0), LPARAM(0)) };
    // Give the main thread time to restore the display before the process ends.
    std::thread::sleep(Duration::from_millis(1500));
    BOOL(1)
}

fn already_running() -> bool {
    // SAFETY: plain named-mutex creation; the handle is intentionally kept
    // open for the lifetime of the process.
    unsafe {
        let handle = CreateMutexW(None, true, w!("Local\\cs2-res-switcher"));
        handle.is_ok() && GetLastError() == ERROR_ALREADY_EXISTS
    }
}

fn main() {
    logger::init();

    if already_running() {
        log_info!("Another instance is already running, exiting");
        // SAFETY: simple modal message box with static strings.
        unsafe {
            MessageBoxW(
                None,
                w!("cs2-res-switcher вже запущено (дивіться іконку в треї)."),
                w!("cs2-res-switcher"),
                MB_OK | MB_ICONINFORMATION,
            );
        }
        return;
    }

    let (cfg, warning) = config::load();
    log_info!(
        "Started. Watching {} -> {}x{} @ {} Hz, poll {} ms",
        cfg.process_name,
        cfg.width,
        cfg.height,
        cfg.refresh_rate,
        cfg.poll_interval_ms
    );
    if let Some(w) = warning {
        log_info!("{w}");
    }

    let tray = match Tray::new() {
        Ok(t) => t,
        Err(e) => {
            log_error!("Cannot create tray icon: {e}");
            return;
        }
    };

    // SAFETY: no preconditions.
    MAIN_THREAD_ID.store(unsafe { GetCurrentThreadId() }, Ordering::SeqCst);
    // SAFETY: handler is a valid `extern "system"` fn for the process lifetime.
    // Fails harmlessly when there is no console (release build).
    let _ = unsafe { SetConsoleCtrlHandler(Some(console_handler), true) };

    let sys = match System::new() {
        Ok(s) => s,
        Err(e) => {
            log_error!("Cannot initialise process monitor: {e}");
            return;
        }
    };

    let poll = Duration::from_millis(cfg.poll_interval_ms);
    let mut app = App {
        target: OsString::from(cfg.process_name.trim()),
        cfg,
        sys,
        switcher: Switcher::default(),
        tray,
        mode: Mode::Idle,
        paused: false,
    };
    app.tick();
    let mut last_poll = Instant::now();

    // SAFETY: thread timer (no window); WM_TIMER is posted to this thread.
    let timer = unsafe { SetTimer(None, 0, TICK_MS, None) };

    let mut msg = MSG::default();
    loop {
        // SAFETY: standard Win32 message loop on the thread that owns the tray.
        let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if ret.0 <= 0 {
            break; // WM_QUIT (0) or error (-1)
        }
        if msg.message == WM_APP_QUIT && msg.hwnd.is_invalid() {
            log_info!("Console close/Ctrl+C");
            app.restore();
            // SAFETY: ends the loop on the next GetMessageW.
            unsafe { PostQuitMessage(0) };
            continue;
        }
        // SAFETY: dispatching a message we just received.
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        if let Some(action) = app.tray.poll_action() {
            if !app.handle(action) {
                // SAFETY: ends the loop on the next GetMessageW.
                unsafe { PostQuitMessage(0) };
            }
        }

        if msg.message == WM_TIMER && last_poll.elapsed() >= poll {
            last_poll = Instant::now();
            app.tick();
        }
    }

    if timer != 0 {
        // SAFETY: killing the thread timer created above.
        let _ = unsafe { KillTimer(None, timer) };
    }
    app.restore();
    log_info!("Exited");
    // `app.switcher` restores again on drop if anything is still switched.
}
