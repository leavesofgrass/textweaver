//! The window's frame follows textweaver's theme (W8a-m): the title bar on
//! every system, and on Windows the menu bar and its drop-down menus.
//!
//! One rule decides, [`chrome`]: Windows high contrast always wins (the
//! system draws the frame in the person's own colors); otherwise the
//! effective palette's background decides, dark below the luminance at
//! which black and white text have equal contrast. With
//! `display.follow_os_theme` on, the system's setting picks the theme at
//! startup, and the frame follows that theme like any other.
//!
//! On Windows, a dark menu needs more than muda's `MenuTheme`: the menu
//! bar is drawn by muda, but the drop-down menus are the system's, and
//! they go dark only when the application asked for dark mode through
//! uxtheme's unnamed exports (`SetPreferredAppMode`, ordinal 135, and
//! `AllowDarkModeForWindow`, ordinal 133) and the menu themes were flushed
//! (`FlushMenuThemes`, ordinal 136). Those are looked up by ordinal, only
//! on builds that have them, and a missing one is skipped silently: the
//! menus then stay light, which is readable, never broken.

use masonry_winit::winit::window::Theme;
use textweaver_theme::Rgb;

use crate::theme::Palette;

/// How the window's frame (title bar and menus) is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chrome {
    /// Dark: a dark title bar, menu bar, and drop-down menus.
    Dark,
    /// Light.
    Light,
    /// The system's high contrast colors: the system draws the frame.
    System,
}

impl Chrome {
    /// The title bar's theme for winit: dark or light, or `None` to leave
    /// it to the system (high contrast).
    pub fn window_theme(self) -> Option<Theme> {
        match self {
            Chrome::Dark => Some(Theme::Dark),
            Chrome::Light => Some(Theme::Light),
            Chrome::System => None,
        }
    }

    /// True for the dark frame.
    pub fn is_dark(self) -> bool {
        self == Chrome::Dark
    }

    /// A word for the log: "dark", "light", or "system".
    pub fn name(self) -> &'static str {
        match self {
            Chrome::Dark => "dark",
            Chrome::Light => "light",
            Chrome::System => "system",
        }
    }
}

/// The one rule: the system's high contrast mode wins; otherwise a dark
/// page (`background`) gets a dark frame and a light one a light frame.
pub fn chrome(background: Rgb, os_high_contrast: bool) -> Chrome {
    if os_high_contrast {
        Chrome::System
    } else if background.is_dark() {
        Chrome::Dark
    } else {
        Chrome::Light
    }
}

/// The frame for the window's effective palette, with the system's high
/// contrast mode read now.
pub fn for_palette(palette: &Palette) -> Chrome {
    chrome(palette.background, os_high_contrast())
}

/// True while the system's high contrast mode is on (Windows only; the
/// other systems' setting picks textweaver's own high-contrast theme at
/// startup instead).
pub fn os_high_contrast() -> bool {
    crate::system_colors::high_contrast().is_some()
}

#[cfg(windows)]
pub use win::{flush_menus, prepare};

#[cfg(windows)]
#[allow(unsafe_code)]
mod win {
    use std::sync::OnceLock;

    use super::Chrome;

    /// A DLL export, as `GetProcAddress` returns it (`FARPROC`).
    type Proc = unsafe extern "system" fn() -> isize;

    windows_core::link!("kernel32.dll" "system" fn LoadLibraryExW(name: *const u16, file: isize, flags: u32) -> isize);
    windows_core::link!("kernel32.dll" "system" fn GetProcAddress(module: isize, name: *const u8) -> Option<Proc>);

    /// Only `System32` is searched, so no other folder's DLL is loaded.
    const LOAD_LIBRARY_SEARCH_SYSTEM32: u32 = 0x0000_0800;

    /// The first build with uxtheme's dark mode exports (Windows 10,
    /// 1809), and the first where ordinal 135 is `SetPreferredAppMode`
    /// (1903; before, it is `AllowDarkModeForApp`).
    const FIRST_DARK_BUILD: u32 = 17_763;
    const FIRST_APP_MODE_BUILD: u32 = 18_362;

    /// `PreferredAppMode`: the system's default, or forced dark or light.
    const APP_MODE_DEFAULT: i32 = 0;
    const APP_MODE_FORCE_DARK: i32 = 2;
    const APP_MODE_FORCE_LIGHT: i32 = 3;

    type SetPreferredAppMode = unsafe extern "system" fn(i32) -> i32;
    type AllowDarkModeForApp = unsafe extern "system" fn(bool) -> bool;
    type AllowDarkModeForWindow = unsafe extern "system" fn(isize, bool) -> bool;
    type FlushMenuThemes = unsafe extern "system" fn();

    /// The exports this build has.
    struct Exports {
        build: u32,
        app_mode: Option<Proc>,
        allow_window: Option<AllowDarkModeForWindow>,
        flush: Option<FlushMenuThemes>,
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// A system DLL, or 0.
    fn system_dll(name: &str) -> isize {
        let name = wide(name);
        // SAFETY: a NUL-terminated name, a null file handle, and a flag
        // that limits the search to System32. The module is kept loaded
        // for the process's life (it is never freed).
        unsafe { LoadLibraryExW(name.as_ptr(), 0, LOAD_LIBRARY_SEARCH_SYSTEM32) }
    }

    /// An export by ordinal.
    fn ordinal(module: isize, n: u16) -> Option<Proc> {
        if module == 0 {
            return None;
        }
        // SAFETY: GetProcAddress takes an ordinal in the low word of the
        // name pointer (MAKEINTRESOURCE); `module` is a loaded module.
        unsafe { GetProcAddress(module, std::ptr::without_provenance(usize::from(n))) }
    }

    /// The Windows build number, from ntdll's `RtlGetVersion` (which, unlike
    /// `GetVersionEx`, is not shimmed by the manifest), or 0.
    fn build() -> u32 {
        #[repr(C)]
        struct OsVersionInfoW {
            size: u32,
            major: u32,
            minor: u32,
            build: u32,
            platform: u32,
            service_pack: [u16; 128],
        }
        type RtlGetVersion = unsafe extern "system" fn(*mut OsVersionInfoW) -> i32;
        let ntdll = system_dll("ntdll.dll");
        if ntdll == 0 {
            return 0;
        }
        // SAFETY: a NUL-terminated ASCII name, and a loaded module.
        let Some(f) = (unsafe { GetProcAddress(ntdll, c"RtlGetVersion".as_ptr().cast()) }) else {
            return 0;
        };
        // SAFETY: RtlGetVersion has exactly this signature.
        let f = unsafe { std::mem::transmute::<Proc, RtlGetVersion>(f) };
        let mut info = OsVersionInfoW {
            size: std::mem::size_of::<OsVersionInfoW>() as u32,
            major: 0,
            minor: 0,
            build: 0,
            platform: 0,
            service_pack: [0; 128],
        };
        // SAFETY: `info` is a live OSVERSIONINFOW with its size set.
        let status = unsafe { f(&mut info) };
        if status >= 0 && info.major >= 10 {
            info.build
        } else {
            0
        }
    }

    fn exports() -> &'static Exports {
        static EXPORTS: OnceLock<Exports> = OnceLock::new();
        EXPORTS.get_or_init(|| {
            let build = build();
            if build < FIRST_DARK_BUILD {
                return Exports {
                    build,
                    app_mode: None,
                    allow_window: None,
                    flush: None,
                };
            }
            let uxtheme = system_dll("uxtheme.dll");
            Exports {
                build,
                app_mode: ordinal(uxtheme, 135),
                // SAFETY (both): the exports at these ordinals have these
                // signatures on every build from 1809 on (checked above).
                allow_window: ordinal(uxtheme, 133)
                    .map(|f| unsafe { std::mem::transmute::<Proc, AllowDarkModeForWindow>(f) }),
                flush: ordinal(uxtheme, 136)
                    .map(|f| unsafe { std::mem::transmute::<Proc, FlushMenuThemes>(f) }),
            }
        })
    }

    /// Asks Windows to draw this application's menus and this window's
    /// frame in `chrome`'s mode: the app's preferred mode (forced dark or
    /// light, or the system's default in high contrast), and dark mode
    /// allowed for the window or not. Call it before the menus are made
    /// and whenever the theme changes, then [`flush_menus`]. Returns false
    /// when this Windows has none of the calls (nothing was done).
    pub fn prepare(hwnd: isize, chrome: Chrome) -> bool {
        let e = exports();
        let dark = chrome.is_dark();
        let mut done = false;
        if let Some(f) = e.app_mode {
            if e.build >= FIRST_APP_MODE_BUILD {
                // SAFETY: from 1903 on, ordinal 135 is SetPreferredAppMode.
                let set = unsafe { std::mem::transmute::<Proc, SetPreferredAppMode>(f) };
                let mode = match chrome {
                    Chrome::Dark => APP_MODE_FORCE_DARK,
                    Chrome::Light => APP_MODE_FORCE_LIGHT,
                    Chrome::System => APP_MODE_DEFAULT,
                };
                // SAFETY: takes and returns a PreferredAppMode by value.
                let _ = unsafe { set(mode) };
            } else {
                // SAFETY: on 1809, ordinal 135 is AllowDarkModeForApp.
                let allow = unsafe { std::mem::transmute::<Proc, AllowDarkModeForApp>(f) };
                // SAFETY: takes and returns a bool by value.
                let _ = unsafe { allow(dark) };
            }
            done = true;
        }
        if let Some(allow) = e.allow_window
            && hwnd != 0
        {
            // SAFETY: `hwnd` is this process's live top-level window.
            let _ = unsafe { allow(hwnd, dark) };
            done = true;
        }
        done
    }

    /// Makes the menus pick up the mode [`prepare`] set: the menu themes
    /// are flushed and the menu bar is drawn again.
    pub fn flush_menus(hwnd: isize) {
        if let Some(flush) = exports().flush {
            // SAFETY: takes nothing and returns nothing.
            unsafe { flush() };
        }
        if hwnd != 0 {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::DrawMenuBar;
            // SAFETY: this process's own window.
            let _ = unsafe { DrawMenuBar(HWND(hwnd as *mut core::ffi::c_void)) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_contrast_wins_over_any_page() {
        let black = Rgb { r: 0, g: 0, b: 0 };
        let white = Rgb {
            r: 255,
            g: 255,
            b: 255,
        };
        assert_eq!(chrome(black, true), Chrome::System);
        assert_eq!(chrome(white, true), Chrome::System);
        assert_eq!(chrome(black, false), Chrome::Dark);
        assert_eq!(chrome(white, false), Chrome::Light);
        assert_eq!(Chrome::System.window_theme(), None);
    }
}
