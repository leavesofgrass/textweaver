//! The public helper programs textweaver can fetch for the reader (beta
//! 1, the owner's "the app fetches helper programs for the user"):
//! ffmpeg (M4B and MP4 export), liblouis (reading braille as print, and
//! contracted braille), and pandoc (more document formats).
//!
//! Each is an optional component like the dictation models: a pinned
//! version from its official release page, with the size, the SHA-256,
//! and a license note, offered only when the reader says yes. Where a
//! publisher has no build for this computer (liblouis on Linux and macOS,
//! ffmpeg on macOS), there is no component: the offer names the system's
//! package command to copy instead ([`Helper::package_command`]), and
//! textweaver finds the program once it is installed.
//!
//! Every lookup checks the components folder first, then the environment
//! variable and the PATH (`textweaver_store::find_helper`), so a reader
//! who installed a helper some other way is never asked.
//!
//! The SHA-256 of each file is the digest GitHub publishes for the
//! release asset, read from the release page on Friday, October 9, 2026;
//! none of the three publishes its own checksum file for these builds.

use std::borrow::Cow;
use std::path::Path;

use textweaver_components::{Action, Check, Component, FilePin, Listing, Platform};

/// A helper program a feature can need.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Helper {
    /// ffmpeg: M4B audiobooks and MP4 video.
    Ffmpeg,
    /// liblouis's `lou_translate`: braille files read as print, and
    /// contracted braille.
    Liblouis,
    /// pandoc: document formats textweaver has no reader for.
    Pandoc,
}

/// One pinned file: its name, size, and SHA-256. Its address is the
/// release's address and the name.
#[derive(Clone, Copy, Debug)]
struct Pin {
    name: &'static str,
    size: u64,
    sha256: &'static str,
}

/// BtbN's ffmpeg builds: one daily autobuild, pinned.
// shortcut: BtbN prunes old autobuilds after a while, so this pin moves
// with each textweaver release; a components source or a mirror keeps an
// older pin working.
const FFMPEG_VERSION: &str = "9.0.2";
const FFMPEG_RELEASE: &str =
    "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-10-09-14-16/";

const FFMPEG_WIN64: Pin = Pin {
    name: "ffmpeg-n9.0.2-24-gfd5d616c29-win64-lgpl-9.0.zip",
    size: 171_475_858,
    sha256: "b729b1a31336336426aafba99f052746e0afb994a1c2527251afb5e4c2372b03",
};
const FFMPEG_WINARM64: Pin = Pin {
    name: "ffmpeg-n9.0.2-24-gfd5d616c29-winarm64-lgpl-9.0.zip",
    size: 118_009_235,
    sha256: "0c1963609313ba4891ea6f49f8014e18e503fce131bf8b8985ae706cb85f0b85",
};
const FFMPEG_LINUX64: Pin = Pin {
    name: "ffmpeg-n9.0.2-24-gfd5d616c29-linux64-lgpl-9.0.tar.xz",
    size: 137_918_664,
    sha256: "17b452d22643d79fab84106496bf361f8d84efcb4548bd70432ffd9fad2c7ebe",
};
const FFMPEG_LINUXARM64: Pin = Pin {
    name: "ffmpeg-n9.0.2-24-gfd5d616c29-linuxarm64-lgpl-9.0.tar.xz",
    size: 117_012_696,
    sha256: "e8cea70b89052c129cebd11fa32362f961575b3212b9857edad69f7b88a3369d",
};

const LIBLOUIS_VERSION: &str = "3.39.0";
const LIBLOUIS_RELEASE: &str = "https://github.com/liblouis/liblouis/releases/download/v3.39.0/";
const LIBLOUIS_WIN64: Pin = Pin {
    name: "liblouis-3.39.0-win64.zip",
    size: 5_762_099,
    sha256: "64d669ac30f1411e0023b1cecc81c7a7b5374678ee41302c95ac8c7c8fbc6591",
};

const PANDOC_VERSION: &str = "3.12.1";
const PANDOC_RELEASE: &str = "https://github.com/jgm/pandoc/releases/download/3.12.1/";
const PANDOC_WIN64: Pin = Pin {
    name: "pandoc-3.12.1-windows-x86_64.zip",
    size: 42_154_914,
    sha256: "6ef431cb20b2c24a0371fc19200dc49d452be54b1b5c9239f3462841ef53e587",
};
const PANDOC_LINUX64: Pin = Pin {
    name: "pandoc-3.12.1-linux-amd64.tar.gz",
    size: 35_363_545,
    sha256: "d0c90410e90204c9ca83b8539fac5c7aed01fd537207e4585849f8abc5df20b8",
};
const PANDOC_LINUXARM64: Pin = Pin {
    name: "pandoc-3.12.1-linux-arm64.tar.gz",
    size: 37_892_300,
    sha256: "445d96fd08801fe636cab1158b4f017ee320ac3b7446328b4fe2f902117b19b9",
};
const PANDOC_MAC_X64: Pin = Pin {
    name: "pandoc-3.12.1-x86_64-macOS.pkg",
    size: 26_440_373,
    sha256: "e657f383500d189721bfb77531d63083f84eefba2117acf6fabe164bbd8a954f",
};
const PANDOC_MAC_ARM64: Pin = Pin {
    name: "pandoc-3.12.1-arm64-macOS.pkg",
    size: 42_174_951,
    sha256: "40205bb2d91dee791b98be2a1ce78a2823491bba329f20b922e4120d9a8b428c",
};

impl Helper {
    /// Every helper, in the order the lists show them.
    pub const ALL: [Helper; 3] = [Helper::Ffmpeg, Helper::Liblouis, Helper::Pandoc];

    /// Its component id, a plain name: `ffmpeg`, `liblouis`, `pandoc`.
    pub const fn id(self) -> &'static str {
        match self {
            Helper::Ffmpeg => "ffmpeg",
            Helper::Liblouis => "liblouis",
            Helper::Pandoc => "pandoc",
        }
    }

    /// The helper whose component id is `id`.
    pub fn from_id(id: &str) -> Option<Helper> {
        Helper::ALL.into_iter().find(|h| h.id() == id)
    }

    /// Its name, as said ("FFmpeg").
    pub const fn name(self) -> &'static str {
        match self {
            Helper::Ffmpeg => "FFmpeg",
            Helper::Liblouis => "liblouis",
            Helper::Pandoc => "Pandoc",
        }
    }

    /// The program textweaver runs.
    pub const fn program(self) -> &'static str {
        match self {
            Helper::Ffmpeg => "ffmpeg",
            Helper::Liblouis => "lou_translate",
            Helper::Pandoc => "pandoc",
        }
    }

    /// The environment variable that names the program, when there is one.
    pub const fn env(self) -> Option<&'static str> {
        match self {
            Helper::Ffmpeg => Some("TEXTWEAVER_FFMPEG"),
            Helper::Liblouis => None,
            Helper::Pandoc => Some("TEXTWEAVER_PANDOC"),
        }
    }

    /// The program, when it is found: in `components` (the components
    /// folder) first, then the environment variable, then the PATH.
    pub fn find_in(self, components: Option<&Path>) -> Option<std::path::PathBuf> {
        textweaver_store::find_helper_in(
            components,
            self.program(),
            self.env().and_then(std::env::var_os),
            std::env::var_os("PATH"),
        )
    }

    /// Its optional component for this computer, when its publisher has a
    /// build for it.
    pub fn component(self) -> Option<Component> {
        self.component_for(Platform::current(), std::env::consts::ARCH)
    }

    /// [`Helper::component`] for `platform` and the processor `arch`
    /// (`x86_64`, `aarch64`).
    pub fn component_for(self, platform: Platform, arch: &str) -> Option<Component> {
        let arm = arch == "aarch64";
        let (release, version, pin, action) = match (self, platform) {
            (Helper::Ffmpeg, Platform::Windows) => (
                FFMPEG_RELEASE,
                FFMPEG_VERSION,
                if arm { FFMPEG_WINARM64 } else { FFMPEG_WIN64 },
                Action::Unpack,
            ),
            (Helper::Ffmpeg, Platform::Linux) if arm || arch == "x86_64" => (
                FFMPEG_RELEASE,
                FFMPEG_VERSION,
                if arm { FFMPEG_LINUXARM64 } else { FFMPEG_LINUX64 },
                Action::Unpack,
            ),
            (Helper::Liblouis, Platform::Windows) => (
                LIBLOUIS_RELEASE,
                LIBLOUIS_VERSION,
                LIBLOUIS_WIN64,
                Action::Unpack,
            ),
            (Helper::Pandoc, Platform::Windows) => (
                PANDOC_RELEASE,
                PANDOC_VERSION,
                PANDOC_WIN64,
                Action::Unpack,
            ),
            (Helper::Pandoc, Platform::Linux) if arm || arch == "x86_64" => (
                PANDOC_RELEASE,
                PANDOC_VERSION,
                if arm { PANDOC_LINUXARM64 } else { PANDOC_LINUX64 },
                Action::Unpack,
            ),
            (Helper::Pandoc, Platform::Macos) => (
                PANDOC_RELEASE,
                PANDOC_VERSION,
                if arm { PANDOC_MAC_ARM64 } else { PANDOC_MAC_X64 },
                Action::Installer,
            ),
            _ => return None,
        };
        let (title, license, credit, features): (_, _, _, &'static [Cow<'static, str>]) =
            match self {
                Helper::Ffmpeg => (
                    "FFmpeg 9.0.2, for M4B and MP4 export",
                    "LGPL-2.1-or-later",
                    "FFmpeg (LGPL-2.1-or-later), the LGPL build by BtbN, https://github.com/BtbN/FFmpeg-Builds",
                    &[Cow::Borrowed("audio-export")],
                ),
                Helper::Liblouis => (
                    "liblouis 3.39.0, for braille translation",
                    "LGPL-2.1-or-later; its tools GPL-3.0-or-later",
                    "liblouis (LGPL-2.1-or-later, tools GPL-3.0-or-later), https://liblouis.io",
                    &[Cow::Borrowed("braille")],
                ),
                Helper::Pandoc => (
                    "Pandoc 3.12.1, for more document formats",
                    "GPL-2.0-or-later",
                    "Pandoc by John MacFarlane (GPL-2.0-or-later), https://pandoc.org",
                    &[Cow::Borrowed("pandoc")],
                ),
            };
        Some(Component {
            id: Cow::Borrowed(self.id()),
            title: Cow::Borrowed(title),
            license: Cow::Borrowed(license),
            credit: Cow::Borrowed(credit),
            features: Cow::Borrowed(features),
            folder: Cow::Owned(format!(
                "{}/{}",
                textweaver_store::COMPONENTS_DIR,
                self.id()
            )),
            files: Cow::Owned(vec![FilePin {
                name: Cow::Borrowed(pin.name),
                url: Cow::Owned(format!("{release}{}", pin.name)),
                size: pin.size,
                check: Check::Sha256(Cow::Borrowed(pin.sha256)),
            }]),
            notice: None,
            listing: Some(Listing {
                version: version.to_owned(),
                platform,
                action,
            }),
        })
    }

    /// The command that installs it from the system's packages, for a
    /// computer with no component for it (liblouis on Linux and macOS,
    /// ffmpeg on macOS): `sudo apt install liblouis-bin`, `brew install
    /// ffmpeg`. `None` when this computer has a component, or its package
    /// manager is not known.
    pub fn package_command(self) -> Option<String> {
        if self.component().is_some() {
            return None;
        }
        let os_release = if cfg!(target_os = "linux") {
            std::fs::read_to_string("/etc/os-release").ok()
        } else {
            None
        };
        package_command_for(self, Platform::current(), os_release.as_deref())
    }
}

/// The package command for `helper` on `platform`; on Linux, from the
/// text of `/etc/os-release` (its `ID` and `ID_LIKE`).
pub fn package_command_for(
    helper: Helper,
    platform: Platform,
    os_release: Option<&str>,
) -> Option<String> {
    let (apt, dnf, pacman, brew) = match helper {
        Helper::Ffmpeg => ("ffmpeg", "ffmpeg-free", "ffmpeg", "ffmpeg"),
        Helper::Liblouis => ("liblouis-bin", "liblouis-utils", "liblouis", "liblouis"),
        Helper::Pandoc => ("pandoc", "pandoc", "pandoc", "pandoc"),
    };
    match platform {
        Platform::Macos => Some(format!("brew install {brew}")),
        Platform::Linux => {
            let mut words = Vec::new();
            for line in os_release?.lines() {
                if let Some(v) = line
                    .strip_prefix("ID=")
                    .or_else(|| line.strip_prefix("ID_LIKE="))
                {
                    words.extend(
                        v.trim_matches('"')
                            .split_whitespace()
                            .map(str::to_ascii_lowercase),
                    );
                }
            }
            let is = |names: &[&str]| words.iter().any(|w| names.contains(&w.as_str()));
            if is(&["debian", "ubuntu"]) {
                Some(format!("sudo apt install {apt}"))
            } else if is(&["fedora", "rhel", "centos"]) {
                Some(format!("sudo dnf install {dnf}"))
            } else if is(&["arch"]) {
                Some(format!("sudo pacman -S {pacman}"))
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_platform_gets_the_publishers_build_or_none() {
        let win = Helper::Ffmpeg
            .component_for(Platform::Windows, "x86_64")
            .unwrap();
        win.check_names().unwrap();
        assert_eq!(win.release(), "ffmpeg-9.0.2");
        assert_eq!(win.action(), Action::Unpack);
        assert_eq!(win.folder, "components/ffmpeg");
        assert!(win.files[0].url.starts_with("https://github.com/BtbN/"));
        assert!(win.files[0].name.ends_with("win64-lgpl-9.0.zip"));
        let arm = Helper::Ffmpeg
            .component_for(Platform::Windows, "aarch64")
            .unwrap();
        assert!(arm.files[0].name.contains("winarm64"));
        assert!(
            Helper::Ffmpeg
                .component_for(Platform::Macos, "aarch64")
                .is_none()
        );
        assert!(
            Helper::Liblouis
                .component_for(Platform::Linux, "x86_64")
                .is_none()
        );
        let louis = Helper::Liblouis
            .component_for(Platform::Windows, "x86_64")
            .unwrap();
        assert_eq!(louis.release(), "liblouis-3.39.0");
        assert!(louis.files[0].url.starts_with("https://github.com/liblouis/"));
        let pkg = Helper::Pandoc
            .component_for(Platform::Macos, "aarch64")
            .unwrap();
        assert_eq!(pkg.action(), Action::Installer);
        assert!(pkg.files[0].name.ends_with("arm64-macOS.pkg"));
        let tar = Helper::Pandoc
            .component_for(Platform::Linux, "x86_64")
            .unwrap();
        assert!(tar.files[0].name.ends_with("linux-amd64.tar.gz"));
        assert!(
            Helper::Pandoc
                .component_for(Platform::Linux, "riscv64")
                .is_none()
        );
        for h in Helper::ALL {
            for p in [Platform::Windows, Platform::Linux, Platform::Macos] {
                if let Some(c) = h.component_for(p, "x86_64") {
                    c.check_names().unwrap();
                    assert!(c.files[0].url.starts_with("https://github.com/"));
                    assert_eq!(Helper::from_id(&c.id), Some(h));
                }
            }
        }
    }

    #[test]
    fn the_package_command_follows_the_distribution() {
        let debian = "PRETTY_NAME=Debian\nID=debian\n";
        let ubuntu = "ID=ubuntu\nID_LIKE=debian\n";
        let fedora = "ID=fedora\n";
        let manjaro = concat!("ID=manjaro\nID_LIKE=", '"', "arch", '"', "\n");
        let l = Helper::Liblouis;
        let cmd = |os: &str| package_command_for(l, Platform::Linux, Some(os));
        assert_eq!(cmd(debian).as_deref(), Some("sudo apt install liblouis-bin"));
        assert_eq!(cmd(ubuntu).as_deref(), Some("sudo apt install liblouis-bin"));
        assert_eq!(cmd(fedora).as_deref(), Some("sudo dnf install liblouis-utils"));
        assert_eq!(cmd(manjaro).as_deref(), Some("sudo pacman -S liblouis"));
        assert_eq!(cmd("ID=gentoo"), None);
        assert_eq!(
            package_command_for(Helper::Ffmpeg, Platform::Macos, None).as_deref(),
            Some("brew install ffmpeg")
        );
        assert_eq!(package_command_for(l, Platform::Windows, None), None);
    }
}
