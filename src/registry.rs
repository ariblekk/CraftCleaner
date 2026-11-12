//! A small cleaner for obviously broken registry entries.
//!
//! Only the three checks whose pointers can be tested safely are here: shortcuts
//! pointing at files that no longer exist, missing startup programs, and
//! uninstall entries whose folder is gone. Nothing is removed without a click,
//! and nothing is removed for good: shortcuts go to the Recycle Bin.

use std::path::PathBuf;

/// Which registry root an entry lives under. The two `Run` keys are separate
/// keys: a name in the machine-wide one says nothing about the per-user one, so
/// a fix has to know which it is holding.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hive {
    CurrentUser,
    LocalMachine,
}

impl Hive {
    /// The short name Windows tools print: `HKCU`, `HKLM`.
    pub fn label(self) -> &'static str {
        match self {
            Hive::CurrentUser => "HKCU",
            Hive::LocalMachine => "HKLM",
        }
    }

    #[cfg(windows)]
    fn predefined(self) -> winreg::HKEY {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        match self {
            Hive::CurrentUser => HKEY_CURRENT_USER,
            Hive::LocalMachine => HKEY_LOCAL_MACHINE,
        }
    }
}

/// What the scan knows how to undo for an issue.
#[derive(Clone, Debug)]
pub enum Fix {
    /// A value under `Run`/`RunOnce` in a given hive.
    Startup {
        hive: Hive,
        key: String,
        value: String,
    },
    /// A `.lnk` file; removing it sends the file to the Recycle Bin.
    Shortcut { path: PathBuf },
}

/// What kind of entry an issue is; the word shown depends on the language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Shortcut,
    Startup,
    Uninstall,
}

/// What is wrong, kept as a value so the sentence can be built per language.
#[derive(Clone, Debug)]
pub enum Detail {
    PointsAt(String),
    ProgramGone(String),
    FolderGone(String),
}

#[derive(Clone, Debug)]
pub struct Issue {
    pub kind: Kind,
    pub label: String,
    pub detail: Detail,
    /// Where it was found, for the report.
    pub source: String,
    /// `None` when the check can only point at a problem, not undo it.
    pub fix: Option<Fix>,
}

fn expand(s: &str) -> String {
    let mut out = s.to_string();
    for (var, val) in [
        (
            "%SystemRoot%",
            std::env::var("SystemRoot").unwrap_or_default(),
        ),
        ("%windir%", std::env::var("SystemRoot").unwrap_or_default()),
        (
            "%USERPROFILE%",
            std::env::var("USERPROFILE").unwrap_or_default(),
        ),
        (
            "%ProgramFiles%",
            std::env::var("ProgramFiles").unwrap_or_default(),
        ),
        (
            "%ProgramFiles(x86)%",
            std::env::var("ProgramFiles(x86)").unwrap_or_default(),
        ),
        (
            "%ProgramData%",
            std::env::var("ProgramData").unwrap_or_default(),
        ),
        (
            "%SystemDrive%",
            std::env::var("SystemDrive").unwrap_or_default(),
        ),
    ] {
        if !val.is_empty() {
            out = out.replace(var, &val);
        }
    }
    out
}

/// Pull the first program path out of a command line, handling quotes.
fn command_path(cmd: &str) -> Option<PathBuf> {
    let cmd = expand(cmd.trim());
    if cmd.is_empty() {
        return None;
    }
    let p = if let Some(rest) = cmd.strip_prefix('"') {
        rest.split('"').next().unwrap_or(rest)
    } else {
        let lower = cmd.to_ascii_lowercase();
        match lower.find(".exe") {
            Some(i) => &cmd[..i + 4],
            None => cmd.split_whitespace().next().unwrap_or(""),
        }
    };
    Some(PathBuf::from(p))
}

#[cfg(windows)]
fn scan_shortcuts(issues: &mut Vec<Issue>) {
    use std::io::Read;
    // .lnk files are binary; the local path is stored as plain UTF-16 text inside.
    let mut stack: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("APPDATA") {
        stack.push(PathBuf::from(p).join("Microsoft\\Windows\\Start Menu"));
    }
    if let Ok(p) = std::env::var("ProgramData") {
        stack.push(PathBuf::from(p).join("Microsoft\\Windows\\Start Menu"));
    }
    if let Ok(p) = std::env::var("USERPROFILE") {
        stack.push(PathBuf::from(p).join("Desktop"));
    }
    let mut seen = 0;
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            if seen > 4000 {
                return;
            }
            seen += 1;
            let path = e.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|x| x.to_str()) != Some("lnk") {
                continue;
            }
            let mut buf = Vec::new();
            if std::fs::File::open(&path)
                .and_then(|mut f| f.read_to_end(&mut buf))
                .is_err()
            {
                continue;
            }
            // Ask the file itself where it points. Hunting for anything that
            // looks like a path catches icon locations, the working directory
            // and leftover installer text - all of which live in the same
            // file, and none of which say whether the shortcut is broken.
            let Some(target) = shortcut_target(&buf) else {
                continue;
            };
            if !target.is_absolute() || target.exists() {
                continue;
            }
            // A network path would mean waiting on a server that may be off;
            // leave those to the shell, which knows how to handle them.
            if target.to_string_lossy().starts_with("\\\\") {
                continue;
            }
            issues.push(Issue {
                kind: Kind::Shortcut,
                label: path
                    .file_stem()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Shortcut".into()),
                detail: Detail::PointsAt(target.display().to_string()),
                source: path.display().to_string(),
                fix: Some(Fix::Shortcut { path: path.clone() }),
            });
        }
    }
}

/// The target recorded in a `.lnk`, read from its LinkInfo block.
///
/// Only the stored local path is used. A shortcut that relies purely on the
/// shell's item list, or on an advertised MSI feature, has no path here - it is
/// left alone rather than guessed at.
fn shortcut_target(buf: &[u8]) -> Option<PathBuf> {
    // ShellLinkHeader: fixed 0x4C bytes, starting with its own size.
    if u32_at(buf, 0)? != 0x4C {
        return None;
    }
    let flags = u32_at(buf, 0x14)?;
    let mut pos = 0x4Cusize;
    if flags & 1 != 0 {
        // HasLinkTargetIDList: a size-prefixed blob we do not need.
        let n = u16_at(buf, pos)? as usize;
        pos = pos.checked_add(2 + n)?;
    }
    if flags & 2 == 0 {
        return None; // no LinkInfo to read
    }
    let li_size = u32_at(buf, pos)? as usize;
    let li_header = u32_at(buf, pos + 4)? as usize;
    let li_flags = u32_at(buf, pos + 8)?;
    if li_flags & 1 == 0 || li_size < 0x1C {
        return None; // no local base path
    }
    let end = pos.checked_add(li_size)?;
    if end > buf.len() {
        return None;
    }
    let ansi_base = u32_at(buf, pos + 0x10)? as usize;
    let ansi_suffix = u32_at(buf, pos + 0x18)? as usize;
    // The unicode offsets only exist in the larger header.
    let (base_off, suffix_off, wide) = if li_header >= 0x24 {
        let wide_base = u32_at(buf, pos + 0x1C)? as usize;
        let wide_suffix = u32_at(buf, pos + 0x20)? as usize;
        if wide_base != 0 {
            (wide_base, wide_suffix, true)
        } else if ansi_base != 0 {
            (ansi_base, ansi_suffix, false)
        } else {
            return None;
        }
    } else if ansi_base != 0 {
        (ansi_base, ansi_suffix, false)
    } else {
        return None;
    };
    let mut target = link_string(buf, pos, base_off, end, wide)?;
    if suffix_off != 0 {
        target.push_str(&link_string(buf, pos, suffix_off, end, wide).unwrap_or_default());
    }
    if target.len() < 3 {
        return None;
    }
    Some(PathBuf::from(target))
}

fn u16_at(buf: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(buf.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
}

/// A NUL-terminated string inside the LinkInfo block, in either encoding.
fn link_string(buf: &[u8], base: usize, offset: usize, end: usize, wide: bool) -> Option<String> {
    let start = base.checked_add(offset)?;
    if start >= end {
        return None;
    }
    if wide {
        let mut out = String::new();
        let mut i = start;
        while i + 1 < end {
            let c = u16::from_le_bytes([buf[i], buf[i + 1]]);
            if c == 0 {
                break;
            }
            out.push(char::from_u32(c as u32)?);
            i += 2;
        }
        Some(out)
    } else {
        let mut stop = start;
        while stop < end && buf[stop] != 0 {
            stop += 1;
        }
        Some(String::from_utf8_lossy(&buf[start..stop]).into_owned())
    }
}

#[cfg(windows)]
fn scan_startup(issues: &mut Vec<Issue>) {
    use winreg::RegKey;
    const RUN: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    const RUN_ONCE: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce";
    let places: [(Hive, &str); 4] = [
        (Hive::CurrentUser, RUN),
        (Hive::LocalMachine, RUN),
        (Hive::CurrentUser, RUN_ONCE),
        (Hive::LocalMachine, RUN_ONCE),
    ];
    for (hive, sub) in places {
        let root = RegKey::predef(hive.predefined());
        let Ok(key) = root.open_subkey(sub) else {
            continue;
        };
        for item in key.enum_values().flatten() {
            let (name, val) = item;
            let v = val.to_string();
            let Some(p) = command_path(&v) else { continue };
            if !p.exists() && p.is_absolute() {
                issues.push(Issue {
                    kind: Kind::Startup,
                    label: name.clone(),
                    detail: Detail::ProgramGone(p.display().to_string()),
                    source: format!("{}\\{}", hive.label(), sub),
                    fix: Some(Fix::Startup {
                        hive,
                        key: sub.to_string(),
                        value: name,
                    }),
                });
            }
        }
    }
}

#[cfg(windows)]
fn scan_uninstall(issues: &mut Vec<Issue>) {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let root = RegKey::predef(HKEY_LOCAL_MACHINE);
    for sub in [
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
    ] {
        let Ok(key) = root.open_subkey(sub) else {
            continue;
        };
        for entry in key.enum_keys().flatten() {
            let Ok(k) = key.open_subkey(&entry) else {
                continue;
            };
            let name: String = k.get_value("DisplayName").unwrap_or_default();
            if name.is_empty() {
                continue;
            }
            let loc: String = k
                .get_value::<String, _>("InstallLocation")
                .unwrap_or_default()
                .trim_matches('"')
                .to_string();
            if loc.is_empty() || loc.len() < 5 {
                continue; // nothing to test
            }
            let path = std::path::Path::new(&loc);
            if path.is_absolute() && !path.exists() {
                issues.push(Issue {
                    kind: Kind::Uninstall,
                    label: name,
                    detail: Detail::FolderGone(loc),
                    source: format!("HKLM\\{}", sub),
                    fix: None,
                });
            }
        }
    }
}

pub fn scan(shortcuts: bool, startup: bool, uninstall: bool) -> Vec<Issue> {
    let mut v = Vec::new();
    #[cfg(windows)]
    {
        if shortcuts {
            scan_shortcuts(&mut v);
        }
        if startup {
            scan_startup(&mut v);
        }
        if uninstall {
            scan_uninstall(&mut v);
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (shortcuts, startup, uninstall);
    }
    v
}

/// Remove a startup value the scan flagged as broken, from the hive it was
/// actually found in. `HKEY_LOCAL_MACHINE` needs administrator rights; a plain
/// user gets `false` rather than a value deleted from the wrong key.
pub fn fix_startup(hive: Hive, key_path: &str, value_name: &str) -> bool {
    #[cfg(windows)]
    {
        use winreg::enums::KEY_SET_VALUE;
        use winreg::RegKey;
        if let Ok(key) =
            RegKey::predef(hive.predefined()).open_subkey_with_flags(key_path, KEY_SET_VALUE)
        {
            return key.delete_value(value_name).is_ok();
        }
        false
    }
    #[cfg(not(windows))]
    {
        let _ = (hive, key_path, value_name);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_quoted_command_line() {
        let p = command_path("\"C:\\Program Files\\App\\app.exe\" --minimized").unwrap();
        assert_eq!(p, PathBuf::from("C:\\Program Files\\App\\app.exe"));
    }

    #[test]
    fn reads_a_bare_command_line_with_arguments() {
        let p = command_path("C:\\Tools\\updater.exe /silent").unwrap();
        assert_eq!(p, PathBuf::from("C:\\Tools\\updater.exe"));
    }

    #[test]
    fn expands_environment_variables() {
        let win = std::env::var("SystemRoot").unwrap_or_default();
        if win.is_empty() {
            return;
        }
        let p = command_path("%SystemRoot%\\System32\\cmd.exe").unwrap();
        assert_eq!(p, PathBuf::from(format!("{}\\System32\\cmd.exe", win)));
    }

    #[test]
    fn the_two_hives_print_differently() {
        assert_eq!(Hive::CurrentUser.label(), "HKCU");
        assert_eq!(Hive::LocalMachine.label(), "HKLM");
    }

    #[cfg(windows)]
    #[test]
    fn removes_from_the_hive_it_was_found_in() {
        use winreg::RegKey;

        const PATH: &str = "Software\\CleanerSelfTest\\fix_startup";
        const NAME: &str = "CleanerSelfTestValue";

        let hkcu = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        // Start from a known state even if an earlier run was interrupted.
        let _ = hkcu.delete_subkey_all(PATH);
        let (key, _) = hkcu.create_subkey(PATH).expect("create test key");
        key.set_value(NAME, &"C:\\does\\not\\exist.exe").unwrap();

        // The machine-wide key does not exist, so this must fail and leave
        // the per-user value alone: that was the bug when the hive was guessed.
        assert!(!fix_startup(Hive::LocalMachine, PATH, NAME));
        assert!(key.get_value::<String, _>(NAME).is_ok());

        assert!(fix_startup(Hive::CurrentUser, PATH, NAME));
        assert!(key.get_value::<String, _>(NAME).is_err());

        drop(key);
        let _ = hkcu.delete_subkey_all(PATH);
    }

    /// Build a minimal `.lnk`: header, then a LinkInfo block holding the given
    /// strings. `header` is LinkInfoHeaderSize; strings are placed after it.
    fn build_link(
        header: u32,
        id_list: Option<&[u8]>,
        ansi: Option<&str>,
        wide: Option<&str>,
        suffix: Option<&str>,
    ) -> Vec<u8> {
        let mut li = vec![0u8; header as usize];
        let mut body: Vec<u8> = Vec::new();
        let put = |s: &str, wide_string: bool, body: &mut Vec<u8>| -> u32 {
            let off = header + body.len() as u32;
            if wide_string {
                for u in s.encode_utf16() {
                    body.extend_from_slice(&u.to_le_bytes());
                }
                body.extend_from_slice(&[0, 0]);
            } else {
                body.extend_from_slice(s.as_bytes());
                body.push(0);
            }
            off
        };
        let ansi_off = ansi.map(|s| put(s, false, &mut body)).unwrap_or(0);
        let suffix_off = suffix.map(|s| put(s, false, &mut body)).unwrap_or(0);
        let wide_off = wide.map(|s| put(s, true, &mut body)).unwrap_or(0);

        li.extend_from_slice(&body);
        let size = li.len() as u32;
        li[0..4].copy_from_slice(&size.to_le_bytes());
        li[4..8].copy_from_slice(&header.to_le_bytes());
        li[8..12].copy_from_slice(&1u32.to_le_bytes()); // VolumeIDAndLocalBasePath
        li[0x10..0x14].copy_from_slice(&ansi_off.to_le_bytes());
        li[0x18..0x1C].copy_from_slice(&suffix_off.to_le_bytes());
        if header >= 0x24 {
            li[0x1C..0x20].copy_from_slice(&wide_off.to_le_bytes());
        }

        let mut b = vec![0u8; 0x4C];
        b[0..4].copy_from_slice(&0x4Cu32.to_le_bytes());
        b[0x14..0x18].copy_from_slice(&if id_list.is_some() { 3u32 } else { 2u32 }.to_le_bytes());
        if let Some(list) = id_list {
            b.extend_from_slice(&(list.len() as u16).to_le_bytes());
            b.extend_from_slice(list);
        }
        b.extend_from_slice(&li);
        b
    }

    #[test]
    fn reads_a_wide_local_path() {
        let b = build_link(
            0x24,
            None,
            None,
            Some("C:\\Program Files\\Gone App\\app.exe"),
            None,
        );
        assert_eq!(
            shortcut_target(&b),
            Some(PathBuf::from("C:\\Program Files\\Gone App\\app.exe"))
        );
    }

    #[test]
    fn joins_the_ansi_path_and_its_suffix() {
        let b = build_link(
            0x1C,
            None,
            Some("C:\\Program Files\\App\\"),
            None,
            Some("app.exe"),
        );
        assert_eq!(
            shortcut_target(&b),
            Some(PathBuf::from("C:\\Program Files\\App\\app.exe"))
        );
    }

    #[test]
    fn steps_over_the_item_list() {
        let b = build_link(
            0x1C,
            Some(&[1, 2, 3, 4]),
            Some("D:\\gone\\thing.exe"),
            None,
            None,
        );
        assert_eq!(
            shortcut_target(&b),
            Some(PathBuf::from("D:\\gone\\thing.exe"))
        );
    }

    #[test]
    fn an_icon_location_is_not_a_target() {
        // A .lnk with no LinkInfo: only an item list and some other text. The
        // old scanner read the first path-looking string and called it broken.
        let mut b = vec![0u8; 0x4C];
        b[0..4].copy_from_slice(&0x4Cu32.to_le_bytes());
        b[0x14..0x18].copy_from_slice(&3u32.to_le_bytes());
        b.extend_from_slice(&[4, 0, 1, 2, 3, 4]);
        b.extend_from_slice(b"C:\\Program Files\\App\\icon.ico\0");
        assert_eq!(shortcut_target(&b), None);
    }

    #[test]
    fn ignores_a_truncated_file() {
        let mut b = build_link(0x1C, None, Some("C:\\gone\\app.exe"), None, None);
        let cut = b.len() - 4;
        b.truncate(cut);
        assert_eq!(shortcut_target(&b), None);
        assert_eq!(shortcut_target(&[]), None);
    }
}
