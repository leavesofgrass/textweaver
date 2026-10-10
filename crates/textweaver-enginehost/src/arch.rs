//! A library's machine architecture, read from its file header, so a
//! backend can start the host built for it (a 32-bit host for a 32-bit
//! library, the x64 host for an x64 library under an ARM64 program).
//! Shared by the engines whose library the user installs (ECI, DECtalk,
//! eSpeak NG).

use std::path::Path;

/// The largest PE header offset read past the first 4 KiB.
const MAX_SCAN: u64 = 64 * 1024 * 1024;

/// A library's (or a program's) machine architecture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arch {
    /// 32-bit x86.
    X86,
    /// x86-64.
    X64,
    /// 64-bit ARM.
    Arm64,
    /// Something else.
    Other,
}

impl Arch {
    /// Words for messages read aloud.
    pub fn describe(self) -> &'static str {
        match self {
            Arch::X86 => "32-bit x86",
            Arch::X64 => "x86-64",
            Arch::Arm64 => "ARM64",
            Arch::Other => "an unsupported architecture",
        }
    }

    /// The architecture this program was built for.
    pub fn current() -> Arch {
        if cfg!(target_arch = "x86") {
            Arch::X86
        } else if cfg!(target_arch = "x86_64") {
            Arch::X64
        } else if cfg!(target_arch = "aarch64") {
            Arch::Arm64
        } else {
            Arch::Other
        }
    }
}

/// The architecture in a PE (Windows) or ELF (Linux) header, from the
/// file's first bytes.
pub fn arch_from_header(bytes: &[u8]) -> Option<Arch> {
    if bytes.starts_with(b"MZ") {
        let at = u32::from_le_bytes(bytes.get(0x3C..0x40)?.try_into().ok()?) as usize;
        if bytes.get(at..at.checked_add(4)?)? != b"PE\0\0" {
            return None;
        }
        let machine = u16::from_le_bytes(bytes.get(at + 4..at + 6)?.try_into().ok()?);
        return Some(match machine {
            0x014C => Arch::X86,
            0x8664 => Arch::X64,
            0xAA64 => Arch::Arm64,
            _ => Arch::Other,
        });
    }
    if bytes.starts_with(b"\x7fELF") {
        let little = *bytes.get(5)? == 1;
        let raw = bytes.get(18..20)?;
        let machine = if little {
            u16::from_le_bytes([raw[0], raw[1]])
        } else {
            u16::from_be_bytes([raw[0], raw[1]])
        };
        return Some(match (bytes.get(4)?, machine) {
            (1, 3) => Arch::X86,
            (2, 62) => Arch::X64,
            (2, 183) => Arch::Arm64,
            _ => Arch::Other,
        });
    }
    None
}

/// The architecture of the library at `path` (reads its header).
pub fn library_arch(path: &Path) -> Option<Arch> {
    use std::io::Read;
    let mut head = Vec::with_capacity(4096);
    std::fs::File::open(path)
        .ok()?
        .take(4096)
        .read_to_end(&mut head)
        .ok()?;
    if let Some(a) = arch_from_header(&head) {
        return Some(a);
    }
    // A PE header beyond the first 4 KiB: read up to it.
    if head.starts_with(b"MZ") && head.len() >= 0x40 {
        let at = u32::from_le_bytes(head[0x3C..0x40].try_into().ok()?) as u64;
        if at < MAX_SCAN {
            let mut all = Vec::new();
            std::fs::File::open(path)
                .ok()?
                .take(at + 6)
                .read_to_end(&mut all)
                .ok()?;
            return arch_from_header(&all);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_program_has_a_known_architecture() {
        assert_ne!(Arch::current(), Arch::Other);
        assert_eq!(Arch::X64.describe(), "x86-64");
    }

    #[test]
    fn a_pe_header_past_the_first_block_is_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("far.dll");
        let mut bytes = vec![0u8; 5000];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3C..0x40].copy_from_slice(&4800u32.to_le_bytes());
        bytes[4800..4804].copy_from_slice(b"PE\0\0");
        bytes[4804..4806].copy_from_slice(&0xAA64u16.to_le_bytes());
        std::fs::write(&path, bytes).unwrap();
        assert_eq!(library_arch(&path), Some(Arch::Arm64));
        assert_eq!(library_arch(&dir.path().join("missing.dll")), None);
    }
}
