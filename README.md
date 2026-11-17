# cs2-res-switcher

**English** | [Українська](README.uk.md)

A lightweight Windows tray utility. When **CS2** (`cs2.exe`) starts, it switches **only your primary monitor** to a game resolution (1280×960 by default). When the game closes, it puts your desktop back exactly as it was. A second monitor is never touched.

## Installation

1. Open **[Releases](../../releases/latest)** and download `cs2-res-switcher-setup-X.Y.Z.exe`.
2. Run it. No administrator rights are needed.
   - **If the browser blocks the download** (Edge: "isn't commonly downloaded"), press `Ctrl+J`, then **"…" → Keep → Show more → Keep anyway**.
   - **If Windows shows a blue SmartScreen window** ("Windows protected your PC"), click **More info → Run anyway**. This happens because the app is new and has no paid code-signing certificate.
3. Pick the setup language. The app will use the same language (Ukrainian or English).
4. Optionally tick **Start with Windows**.
5. Done: a small monitor icon appears in the tray next to the clock.

To uninstall: **Settings → Apps → cs2-res-switcher → Uninstall**.

## Usage

| Tray icon colour | Meaning |
|---|---|
| grey | waiting for the game |
| green | game resolution is active |
| yellow | paused |

Right-click the icon to open the menu:

- **Status**: shows what the app is doing right now.
- **Pause / Resume**: temporarily stops automatic switching.
- **Switch manually**: switches without the game, handy for a quick test. Click again to restore the desktop.
- **Мова / Language**: choose **Українська** or **English**. The choice is saved.
- **Exit**: always restores your desktop resolution before closing.

## Settings (`config.toml`)

The file is created on first run next to the program. To open it, use **Start → cs2-res-switcher → config.toml**. After editing it, exit the app from the tray and start it again.

```toml
process_name = "cs2.exe"   # game to watch
width = 1280
height = 960
refresh_rate = 0           # 0 = highest your monitor supports, or an exact value such as 240
poll_interval_ms = 1000    # how often to check whether the game is running
scaling = "stretch"        # "stretch" = stretched 4:3, "center" = black bars, "driver" = don't touch
language = "auto"          # "auto" = Windows language, "uk" = Ukrainian, "en" = English
```

If the resolution doesn't switch, open `cs2-res-switcher.log` next to the program. It lists the refresh rates your monitor supports at that resolution.

## Making the switch faster

- **NVIDIA:** NVIDIA Control Panel → *Adjust desktop size and position* → scaling mode *Full-screen*, *Perform scaling on: **GPU***, tick *Override the scaling mode set by games and programs*. The monitor then keeps its native signal and the black screen almost disappears.
- **AMD:** Adrenalin → Display → *GPU Scaling: On*, *Scaling Mode: Full panel*.
- **Windows 11:** Settings → System → Display → Graphics → *Optimizations for windowed games* → On.

## How it works

- The primary monitor is found with `EnumDisplayDevicesW` (`DISPLAY_DEVICE_PRIMARY_DEVICE`).
- The current mode is saved with `EnumDisplaySettingsW(ENUM_CURRENT_SETTINGS)` and restored exactly.
- The switch uses `ChangeDisplaySettingsExW` with flag `0`. The change is temporary and is never written to the registry, so even after a crash a reboot brings back your normal resolution.
- The game process is checked with `sysinfo` once per second, so CPU usage stays practically at zero.
- The app collects no data and makes no network connections.

## Building from source

You need [Rust](https://rustup.rs). For the installer you also need [Inno Setup 6](https://jrsoftware.org/isinfo.php).

```bash
cargo build --release
```

```bash
powershell -ExecutionPolicy Bypass -File build-installer.ps1
```

The installer appears in the `dist` folder.

## License

MIT
