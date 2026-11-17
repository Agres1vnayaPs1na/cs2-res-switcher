//! Primary-monitor display mode handling via the Win32 GDI API.
//!
//! Only the device flagged `DISPLAY_DEVICE_PRIMARY_DEVICE` is ever touched.
//! Changes use `dwflags = 0` (dynamic, not written to the registry), so a crash
//! or reboot falls back to the normal desktop mode stored in the registry.

use crate::{log_error, log_info};
use std::mem::size_of;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsExW, EnumDisplayDevicesW, EnumDisplaySettingsW, CDS_TEST, CDS_TYPE,
    DEVMODEW, DISPLAY_DEVICEW, DISPLAY_DEVICE_PRIMARY_DEVICE, DISP_CHANGE,
    DISP_CHANGE_BADDUALVIEW, DISP_CHANGE_BADFLAGS, DISP_CHANGE_BADMODE, DISP_CHANGE_BADPARAM,
    DISP_CHANGE_FAILED, DISP_CHANGE_NOTUPDATED, DISP_CHANGE_RESTART, DISP_CHANGE_SUCCESSFUL,
    DMDFO_CENTER, DMDFO_STRETCH, DM_BITSPERPEL, DM_DISPLAYFIXEDOUTPUT, DM_DISPLAYFREQUENCY,
    DM_PELSHEIGHT, DM_PELSWIDTH, ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_MODE,
};

use crate::config::Scaling;

/// The primary display adapter output, e.g. `\\.\DISPLAY1`.
struct PrimaryDisplay {
    /// NUL-terminated UTF-16 device name.
    name: Vec<u16>,
    label: String,
}

impl PrimaryDisplay {
    fn pcwstr(&self) -> PCWSTR {
        PCWSTR(self.name.as_ptr())
    }
}

fn wide_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

fn find_primary() -> Result<PrimaryDisplay, String> {
    for index in 0u32.. {
        let mut dd = DISPLAY_DEVICEW {
            cb: size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        // SAFETY: `dd` is a properly sized, writable DISPLAY_DEVICEW.
        let ok = unsafe { EnumDisplayDevicesW(PCWSTR::null(), index, &mut dd, 0) };
        if !ok.as_bool() {
            break;
        }
        if (dd.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE).0 != 0 {
            let end = dd.DeviceName.iter().position(|&c| c == 0).unwrap_or(dd.DeviceName.len());
            let mut name = dd.DeviceName[..end].to_vec();
            name.push(0);
            let label = format!(
                "{} ({})",
                wide_to_string(&dd.DeviceName),
                wide_to_string(&dd.DeviceString)
            );
            return Ok(PrimaryDisplay { name, label });
        }
    }
    Err("primary display device not found".into())
}

fn empty_devmode() -> DEVMODEW {
    DEVMODEW {
        dmSize: size_of::<DEVMODEW>() as u16,
        ..Default::default()
    }
}

fn current_mode(display: &PrimaryDisplay) -> Result<DEVMODEW, String> {
    let mut dm = empty_devmode();
    // SAFETY: valid device name and writable DEVMODEW with dmSize set.
    let ok = unsafe { EnumDisplaySettingsW(display.pcwstr(), ENUM_CURRENT_SETTINGS, &mut dm) };
    if ok.as_bool() {
        Ok(dm)
    } else {
        Err(format!("EnumDisplaySettingsW(CURRENT) failed for {}", display.label))
    }
}

fn all_modes(display: &PrimaryDisplay) -> Vec<DEVMODEW> {
    let mut modes = Vec::new();
    for index in 0u32.. {
        let mut dm = empty_devmode();
        // SAFETY: as above; iteration stops when the API returns FALSE.
        let ok = unsafe {
            EnumDisplaySettingsW(display.pcwstr(), ENUM_DISPLAY_SETTINGS_MODE(index), &mut dm)
        };
        if !ok.as_bool() {
            break;
        }
        modes.push(dm);
    }
    modes
}

/// One resolution the primary monitor supports, with its refresh rates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
    /// Supported refresh rates, highest first.
    pub rates: Vec<u32>,
}

/// Resolutions (>= 800x600, 32 bpp) the primary monitor supports, largest first.
pub fn supported_resolutions() -> Result<Vec<Resolution>, String> {
    let display = find_primary()?;
    let mut list: Vec<Resolution> = Vec::new();
    for m in all_modes(&display) {
        if m.dmBitsPerPel != 32 || m.dmPelsWidth < 800 || m.dmPelsHeight < 600 {
            continue;
        }
        let hz = m.dmDisplayFrequency;
        match list
            .iter_mut()
            .find(|r| r.width == m.dmPelsWidth && r.height == m.dmPelsHeight)
        {
            Some(r) => {
                if !r.rates.contains(&hz) {
                    r.rates.push(hz);
                }
            }
            None => list.push(Resolution {
                width: m.dmPelsWidth,
                height: m.dmPelsHeight,
                rates: vec![hz],
            }),
        }
    }
    for r in &mut list {
        r.rates.sort_unstable_by_key(|&hz| std::cmp::Reverse(hz));
    }
    list.sort_by_key(|r| std::cmp::Reverse((r.width, r.height)));
    Ok(list)
}

fn describe(dm: &DEVMODEW) -> String {
    format!(
        "{}x{} @ {} Hz, {} bpp",
        dm.dmPelsWidth, dm.dmPelsHeight, dm.dmDisplayFrequency, dm.dmBitsPerPel
    )
}

fn disp_change_name(code: DISP_CHANGE) -> &'static str {
    match code {
        DISP_CHANGE_SUCCESSFUL => "DISP_CHANGE_SUCCESSFUL",
        DISP_CHANGE_RESTART => "DISP_CHANGE_RESTART",
        DISP_CHANGE_FAILED => "DISP_CHANGE_FAILED",
        DISP_CHANGE_BADMODE => "DISP_CHANGE_BADMODE",
        DISP_CHANGE_NOTUPDATED => "DISP_CHANGE_NOTUPDATED",
        DISP_CHANGE_BADFLAGS => "DISP_CHANGE_BADFLAGS",
        DISP_CHANGE_BADPARAM => "DISP_CHANGE_BADPARAM",
        DISP_CHANGE_BADDUALVIEW => "DISP_CHANGE_BADDUALVIEW",
        _ => "DISP_CHANGE_UNKNOWN",
    }
}

fn change(display: &PrimaryDisplay, mode: Option<&DEVMODEW>, flags: CDS_TYPE) -> DISP_CHANGE {
    // SAFETY: device name is NUL-terminated; mode (if any) is a valid DEVMODEW.
    unsafe {
        ChangeDisplaySettingsExW(
            display.pcwstr(),
            mode.map(|m| m as *const DEVMODEW),
            None,
            flags,
            None,
        )
    }
}

struct SavedMode {
    display: PrimaryDisplay,
    mode: DEVMODEW,
}

/// Owns the "game mode" state. Restores the saved desktop mode on drop.
#[derive(Default)]
pub struct Switcher {
    saved: Option<SavedMode>,
}

impl Switcher {
    pub fn is_switched(&self) -> bool {
        self.saved.is_some()
    }

    /// Switch the primary monitor to `width`x`height` @ `hz`.
    /// The current mode is saved first so it can be restored exactly.
    pub fn apply(
        &mut self,
        width: u32,
        height: u32,
        hz: u32,
        scaling: Scaling,
    ) -> Result<(), String> {
        if self.saved.is_some() {
            return Ok(());
        }

        let display = find_primary()?;
        let current = current_mode(&display)?;
        log_info!("Primary monitor {}: current {}", display.label, describe(&current));

        let modes = all_modes(&display);
        let same_res: Vec<&DEVMODEW> = modes
            .iter()
            .filter(|m| m.dmPelsWidth == width && m.dmPelsHeight == height)
            .collect();
        if same_res.is_empty() {
            return Err(format!(
                "{width}x{height} is not supported by {}",
                display.label
            ));
        }

        // hz == 0 means "highest refresh rate available at this resolution".
        let hz = if hz == 0 {
            same_res.iter().map(|m| m.dmDisplayFrequency).max().unwrap_or(0)
        } else {
            hz
        };
        let target = same_res
            .iter()
            .filter(|m| m.dmDisplayFrequency == hz)
            .max_by_key(|m| (m.dmBitsPerPel == current.dmBitsPerPel, m.dmBitsPerPel))
            .copied();
        let Some(target) = target else {
            let mut rates: Vec<u32> = same_res.iter().map(|m| m.dmDisplayFrequency).collect();
            rates.sort_unstable();
            rates.dedup();
            return Err(format!(
                "{width}x{height} @ {hz} Hz is not supported by {}; available rates: {rates:?}",
                display.label
            ));
        };

        let mut wanted = *target;
        wanted.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY | DM_BITSPERPEL;

        // Ask the driver to scale the lower mode itself (GPU scaling). When the
        // driver honours it the panel keeps its native timing and doesn't have
        // to re-sync, which is what causes the long black screen.
        let fixed_output = match scaling {
            Scaling::Driver => None,
            Scaling::Stretch => Some(DMDFO_STRETCH),
            Scaling::Center => Some(DMDFO_CENTER),
        };
        if let Some(fo) = fixed_output {
            let mut scaled = wanted;
            scaled.dmFields |= DM_DISPLAYFIXEDOUTPUT;
            scaled.Anonymous1.Anonymous2.dmDisplayFixedOutput = fo;
            if change(&display, Some(&scaled), CDS_TEST) == DISP_CHANGE_SUCCESSFUL {
                wanted = scaled;
            } else {
                log_info!("Driver rejected scaling {scaling:?}; switching without it");
            }
        }

        let test = change(&display, Some(&wanted), CDS_TEST);
        if test != DISP_CHANGE_SUCCESSFUL {
            return Err(format!(
                "test of {} rejected: {} ({})",
                describe(&wanted),
                disp_change_name(test),
                test.0
            ));
        }

        let started = std::time::Instant::now();
        let result = change(&display, Some(&wanted), CDS_TYPE(0));
        log_info!(
            "Switch to {} (scaling {scaling:?}): {} ({}) in {} ms",
            describe(&wanted),
            disp_change_name(result),
            result.0,
            started.elapsed().as_millis()
        );
        if result != DISP_CHANGE_SUCCESSFUL {
            return Err(format!("switch failed: {}", disp_change_name(result)));
        }

        self.saved = Some(SavedMode {
            display,
            mode: current,
        });
        Ok(())
    }

    /// Restore the mode that was active before `apply`. No-op if not switched.
    pub fn restore(&mut self) -> Result<(), String> {
        let Some(saved) = self.saved.take() else {
            return Ok(());
        };

        let result = change(&saved.display, Some(&saved.mode), CDS_TYPE(0));
        log_info!(
            "Restore {} on {}: {} ({})",
            describe(&saved.mode),
            saved.display.label,
            disp_change_name(result),
            result.0
        );
        if result == DISP_CHANGE_SUCCESSFUL {
            return Ok(());
        }

        // Fallback: reload the mode stored in the registry (the desktop mode,
        // since we never write the game mode there).
        let fallback = change(&saved.display, None, CDS_TYPE(0));
        log_info!(
            "Fallback restore from registry: {} ({})",
            disp_change_name(fallback),
            fallback.0
        );
        if fallback == DISP_CHANGE_SUCCESSFUL {
            Ok(())
        } else {
            Err(format!(
                "restore failed: {} / fallback {}",
                disp_change_name(result),
                disp_change_name(fallback)
            ))
        }
    }
}

impl Drop for Switcher {
    fn drop(&mut self) {
        if let Err(e) = self.restore() {
            log_error!("Restore on shutdown failed: {e}");
        }
    }
}
