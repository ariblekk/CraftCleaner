//! Finding and removing junk files.
//!
//! Clean room: everything here is written from the published description of what a
//! disk cleaner does (scan a set of well-known temporary locations, report sizes,
//! delete on request). Nothing was read from any existing program.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::rules::AppRule;

/// One group of files the user can tick on or off.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    WindowsTemp,
    UserTemp,
    RecycleBin,
    ThumbnailCache,
    Prefetch,
    FontCache,
    ShaderCache,
    WindowsUpdateCache,
    DeliveryOptimization,
    ErrorReports,
    BrowserCache,
    BrowserCookies,
    AppCaches,
    RecentDocs,
    WindowsLogs,
    MemoryDumps,
    Downloads, // only leftovers that look like installers/archives older than 7 days
    WindowsOld,
    /// The folders the user added by hand. Their contents are emptied; the
    /// folders themselves stay.
    Custom,
    /// A loaded application rule (see `rules.rs`). The individual program is
    /// identified by `Found::rule`, not by this group.
    AppRule,
}

impl Group {
    pub const ALL: [Group; 20] = [
        Group::WindowsTemp,
        Group::UserTemp,
        Group::RecycleBin,
        Group::ThumbnailCache,
        Group::Prefetch,
        Group::FontCache,
        Group::ShaderCache,
        Group::WindowsUpdateCache,
        Group::DeliveryOptimization,
        Group::ErrorReports,
        Group::BrowserCache,
        Group::BrowserCookies,
        Group::AppCaches,
        Group::RecentDocs,
        Group::WindowsLogs,
        Group::MemoryDumps,
        Group::Downloads,
        Group::WindowsOld,
        Group::Custom,
        Group::AppRule,
    ];

    /// How many groups there are, for the `[bool; Group::COUNT]` of ticks.
    pub const COUNT: usize = Group::ALL.len();

    /// Which tab of the left-hand list this group belongs to. (Unused for now; the
    /// page builds its layout from `layout_for` so headings can be chosen freely.)
    #[allow(dead_code)]
    pub fn tab(self) -> Tab {
        match self {
            Group::BrowserCache | Group::BrowserCookies | Group::AppCaches => Tab::Applications,
            Group::WindowsUpdateCache
            | Group::DeliveryOptimization
            | Group::WindowsOld
            | Group::Prefetch
            | Group::FontCache
            | Group::ShaderCache => Tab::Advanced,
            _ => Tab::Windows,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Windows,
    Applications,
    Advanced,
}

impl Tab {
    pub const ALL: [Tab; 3] = [Tab::Windows, Tab::Applications, Tab::Advanced];
}

/// What one scan found.
#[derive(Default, Clone)]
pub struct Report {
    pub per_group: Vec<(Group, u64, usize)>,
    pub files: Vec<Found>,
}

#[derive(Clone)]
pub struct Found {
    pub path: PathBuf,
    pub size: u64,
    pub group: Group,
    /// Which loaded application rule produced this entry, when it came from one.
    pub rule: Option<usize>,
    /// A directory we only want to empty out, not delete outright.
    pub is_dir: bool,
    /// The folder roots this entry was found under, resolved at scan time.
    ///
    /// The deletion guard normally re-resolves a group's roots, which is right for
    /// the fixed groups. The custom folders are the exception: the user can remove
    /// one from the list between the scan and the click, and the guard would then
    /// find nothing and refuse a deletion the user did ask for. Keeping the roots
    /// that were scanned means the guard still checks "was this inside what we
    /// looked at", without needing the settings to be unchanged.
    pub anchors: Vec<PathBuf>,
}

impl Report {
    pub fn group_size(&self, g: Group) -> u64 {
        self.per_group
            .iter()
            .find(|(k, _, _)| *k == g)
            .map(|(_, b, _)| *b)
            .unwrap_or(0)
    }
    pub fn group_count(&self, g: Group) -> usize {
        self.per_group
            .iter()
            .find(|(k, _, _)| *k == g)
            .map(|(_, _, n)| *n)
            .unwrap_or(0)
    }
    pub fn total(&self) -> u64 {
        self.per_group.iter().map(|(_, b, _)| *b).sum()
    }
    pub fn total_files(&self) -> usize {
        self.files.len()
    }
    /// Bytes a given selection would free.
    pub fn selected_total(&self, want: &dyn Fn(&Found) -> bool) -> u64 {
        self.files.iter().filter(|f| want(f)).map(|f| f.size).sum()
    }

    /// Bytes and items found by one application rule.
    pub fn rule_totals(&self, rule: usize) -> (u64, usize) {
        let mut bytes = 0u64;
        let mut n = 0usize;
        for f in self.files.iter().filter(|f| f.rule == Some(rule)) {
            bytes += f.size;
            n += 1;
        }
        (bytes, n)
    }

    /// Rebuild the per-group totals from the entries still listed.
    pub fn recount(&mut self) {
        let mut per = Vec::new();
        for &g in Group::ALL.iter() {
            let files: Vec<&Found> = self.files.iter().filter(|f| f.group == g).collect();
            if files.is_empty() {
                continue;
            }
            per.push((g, files.iter().map(|f| f.size).sum::<u64>(), files.len()));
        }
        self.per_group = per;
    }
}

const WEEK: u64 = 7 * 24 * 3600;

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn env_dir(key: &str) -> Option<PathBuf> {
    std::env::var_os(key).map(PathBuf::from)
}

/// Well-known folders for a group, skipping any that do not exist.
pub fn roots(g: Group) -> Vec<PathBuf> {
    let win = env_dir("SystemRoot").unwrap_or_else(|| PathBuf::from("C:\\Windows"));
    let mut out = Vec::new();
    let push = |p: PathBuf, out: &mut Vec<PathBuf>| {
        if p.is_dir() {
            out.push(p);
        }
    };
    match g {
        Group::WindowsTemp => push(win.join("Temp"), &mut out),
        Group::UserTemp => {
            if let Some(t) = env_dir("TEMP") {
                push(t, &mut out);
            } else if let Some(u) = env_dir("USERPROFILE") {
                push(u.join("AppData\\Local\\Temp"), &mut out);
            }
        }
        Group::RecycleBin => {
            // One bin per fixed drive, and it sits at the root of the drive —
            // C:\$Recycle.Bin, never %SystemRoot%\$Recycle.Bin. Inside it each
            // account has a folder named after its SID; the scan descends into
            // those it may read and reports the `$R…` items, which are the files
            // and folders the user actually deleted.
            for drive in fixed_drives() {
                push(drive.join("$Recycle.Bin"), &mut out);
            }
        }
        Group::ThumbnailCache => {
            if let Some(u) = env_dir("USERPROFILE") {
                push(
                    u.join("AppData\\Local\\Microsoft\\Windows\\Explorer"),
                    &mut out,
                );
            }
        }
        Group::WindowsUpdateCache => push(win.join("SoftwareDistribution\\Download"), &mut out),
        Group::DeliveryOptimization => push(
            win.join("SoftwareDistribution\\DeliveryOptimization"),
            &mut out,
        ),
        Group::Prefetch => push(win.join("Prefetch"), &mut out),
        Group::FontCache => {
            push(
                win.join("ServiceProfiles\\LocalService\\AppData\\Local\\FontCache"),
                &mut out,
            );
            if let Some(u) = env_dir("USERPROFILE") {
                push(u.join("AppData\\Local\\FontCache"), &mut out);
            }
        }
        Group::ShaderCache => {
            if let Some(u) = env_dir("USERPROFILE") {
                for b in [
                    "AppData\\Local\\D3DSCache",
                    "AppData\\Local\\NVIDIA\\DXCache",
                    "AppData\\Local\\NVIDIA\\GLCache",
                    "AppData\\Local\\AMD\\DxCache",
                    "AppData\\Local\\AMD\\DxcCache",
                ] {
                    push(u.join(b), &mut out);
                }
            }
        }
        Group::AppCaches => {
            if let Some(u) = env_dir("USERPROFILE") {
                for b in [
                    "AppData\\Local\\Packages\\Microsoft.WindowsStore_8wekyb3d8bbwe\\LocalCache",
                    "AppData\\Roaming\\Microsoft\\Teams\\Cache",
                    "AppData\\Roaming\\Microsoft\\Teams\\GPUCache",
                    "AppData\\Local\\Discord\\Cache",
                    "AppData\\Local\\Discord\\Code Cache",
                    "AppData\\Local\\Discord\\GPUCache",
                    "AppData\\Roaming\\Slack\\Cache",
                    "AppData\\Roaming\\Slack\\Code Cache",
                    "AppData\\Roaming\\Slack\\GPUCache",
                ] {
                    push(u.join(b), &mut out);
                }
            }
        }
        Group::WindowsOld => {
            if let Some(sys) = env_dir("SystemDrive") {
                let root = root_of(&sys);
                for b in ["Windows.old", "$WINDOWS.~BT", "$WINDOWS.~WS"] {
                    push(root.join(b), &mut out);
                }
            }
        }
        Group::Custom => {}
        // Application rules bring their own folders; the group itself has none.
        Group::AppRule => {}
        Group::ErrorReports => {
            push(win.join("Minidump"), &mut out);
            push(win.join("LiveKernelReports"), &mut out);
            if let Some(u) = env_dir("USERPROFILE") {
                push(u.join("AppData\\Local\\CrashDumps"), &mut out);
            }
            if let Some(p) = env_dir("ProgramData") {
                push(p.join("Microsoft\\Windows\\WER"), &mut out);
            }
        }
        Group::BrowserCache => {
            if let Some(u) = env_dir("USERPROFILE") {
                for b in [
                    "AppData\\Local\\Google\\Chrome\\User Data\\Default\\Cache",
                    "AppData\\Local\\Microsoft\\Edge\\User Data\\Default\\Cache",
                    "AppData\\Roaming\\Mozilla\\Firefox\\Profiles",
                    "AppData\\Local\\BraveSoftware\\Brave-Browser\\User Data\\Default\\Cache",
                    "AppData\\Local\\Vivaldi\\User Data\\Default\\Cache",
                    "AppData\\Local\\Microsoft\\Windows\\INetCache",
                ] {
                    push(u.join(b), &mut out);
                }
            }
        }
        Group::BrowserCookies => {
            if let Some(u) = env_dir("USERPROFILE") {
                for b in [
                    "AppData\\Local\\Google\\Chrome\\User Data\\Default\\Network",
                    "AppData\\Local\\Microsoft\\Edge\\User Data\\Default\\Network",
                    "AppData\\Local\\BraveSoftware\\Brave-Browser\\User Data\\Default\\Network",
                    "AppData\\Local\\Vivaldi\\User Data\\Default\\Network",
                    "AppData\\Roaming\\Mozilla\\Firefox\\Profiles",
                ] {
                    push(u.join(b), &mut out);
                }
            }
        }
        Group::RecentDocs => {
            if let Some(u) = env_dir("USERPROFILE") {
                push(
                    u.join("AppData\\Roaming\\Microsoft\\Windows\\Recent"),
                    &mut out,
                );
            }
        }
        Group::WindowsLogs => {
            push(win.join("Logs"), &mut out);
            push(win.join("Panther"), &mut out);
        }
        Group::MemoryDumps => push(win.join("MEMORY.DMP"), &mut out),
        Group::Downloads => {
            if let Some(u) = env_dir("USERPROFILE") {
                push(u.join("Downloads"), &mut out);
            }
        }
    }
    out
}

/// Fixed drives, each as a root like `C:\`, so the per-drive folders are found.
fn fixed_drives() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(sys) = env_dir("SystemDrive") {
        out.push(root_of(&sys));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        const DRIVE_FIXED: u32 = 3;
        for letter in b'A'..=b'Z' {
            let root = PathBuf::from(format!("{}:\\", letter as char));
            let wide: Vec<u16> = std::ffi::OsStr::new(&root)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            // A mapped share, a card reader or an empty drive answers something
            // else, and asking a disconnected share for its bin can hang.
            if unsafe { GetDriveTypeW(wide.as_ptr()) } == DRIVE_FIXED && !out.contains(&root) {
                out.push(root);
            }
        }
    }
    #[cfg(not(windows))]
    out.push(PathBuf::from("/"));
    out
}

/// `C:` or `C:\Users\x` becomes `C:\`; the recycle bin needs the drive root.
fn root_of(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        out.push(c.as_os_str());
        if matches!(c, std::path::Component::RootDir) {
            break;
        }
    }
    if !out.has_root() {
        out.push("\\");
    }
    out
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetDriveTypeW(root: *const u16) -> u32;
}

/// Does this group have anywhere to look on this machine?
pub fn available(g: Group) -> bool {
    !roots(g).is_empty()
}

/// Folders for a group, with the user's own folders for `Group::Custom`.
///
/// Folders that do not exist are dropped, so an unplugged drive or a folder
/// since renamed simply finds nothing instead of failing.
pub fn roots_of(g: Group, custom: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = if g == Group::Custom {
        Vec::new()
    } else {
        roots(g)
    };
    if g == Group::Custom {
        for c in custom {
            if c.is_dir() && !out.contains(c) {
                out.push(c.clone());
            }
        }
    }
    out
}

#[allow(dead_code)]
fn file_size(p: &Path) -> u64 {
    std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
}

fn old_enough(p: &Path, secs: u64) -> bool {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| now_unix().saturating_sub(d.as_secs()) > secs)
        .unwrap_or(false)
}

fn interesting_download(p: &Path) -> bool {
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "exe" | "msi" | "zip" | "7z" | "rar" | "iso" | "dmg" | "pkg"
    )
}

/// Walk one group and collect candidates. Bounded so a huge folder cannot hang the app.
///
/// `custom` holds the user's own folders and is only used by `Group::Custom`;
/// every other group reads its own well-known list.
fn scan_group(
    g: Group,
    custom: &[PathBuf],
    exclude: &[PathBuf],
    budget: &mut usize,
    files: &mut Vec<Found>,
    seen: &mut HashSet<PathBuf>,
) -> (u64, usize) {
    let mut bytes = 0u64;
    let mut count = 0usize;
    let roots = roots_of(g, custom)
        .into_iter()
        .filter(|r| !under_any(r, exclude))
        .collect::<Vec<_>>();
    if g == Group::WindowsOld {
        // Far too big to list file by file, and the whole leftover folder is the
        // thing to remove: one entry per leftover, sized as a whole.
        for r in &roots {
            if *budget == 0 {
                break;
            }
            let sz = dir_size(r, budget);
            bytes += sz;
            count += 1;
            files.push(Found {
                path: r.clone(),
                size: sz,
                group: g,
                rule: None,
                is_dir: true,
                anchors: vec![r.clone()],
            });
        }
        return (bytes, count);
    }
    let mut stack: Vec<(PathBuf, PathBuf)> = roots.iter().map(|r| (r.clone(), r.clone())).collect();
    // Folders already read. A group can name the same place twice (`…\Temp` and
    // one of its own children), and listing it twice would count it twice.
    let mut seen_dirs: Vec<PathBuf> = Vec::new();
    while let Some((dir, root)) = stack.pop() {
        if seen_dirs.contains(&dir) {
            continue;
        }
        seen_dirs.push(dir.clone());
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue, // in use or not permitted: skip quietly
        };
        for e in entries.flatten() {
            if *budget == 0 {
                return (bytes, count);
            }
            let path = e.path();
            let meta = match e.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            // An excluded folder takes its whole tree out of the scan, so nothing
            // under it can ever reach a report that the cleaner would act on.
            if !exclude.is_empty() && under_any(&path, exclude) {
                continue;
            }
            if meta.is_dir() {
                if g == Group::MemoryDumps {
                    continue;
                }
                if g == Group::Custom {
                    // A folder inside one of the user's folders goes as a whole, so
                    // the folder is emptied rather than left full of empty shells.
                    let sz = dir_size(&path, budget);
                    bytes += sz;
                    count += 1;
                    files.push(Found {
                        path,
                        size: sz,
                        group: g,
                        rule: None,
                        is_dir: true,
                        anchors: vec![root.clone()],
                    });
                    continue;
                }
                if g == Group::RecycleBin {
                    // Inside $Recycle.Bin: the folders named after an account hold
                    // the items, and an item that was a folder is one entry.
                    let name = file_name_lower(&path);
                    if name.starts_with("$r") {
                        let sz = dir_size(&path, budget);
                        bytes += sz;
                        count += 1;
                        files.push(Found {
                            path,
                            size: sz,
                            group: g,
                            rule: None,
                            is_dir: true,
                            anchors: vec![root.clone()],
                        });
                    } else if !name.starts_with('$') && stack.len() < 4096 {
                        stack.push((path, root.clone()));
                    }
                    continue;
                }
                if stack.len() < 4096 {
                    stack.push((path, root.clone()));
                }
                continue;
            }
            if !wanted_file(g, &path) || !seen.insert(path.clone()) {
                continue;
            }
            let sz = meta.len();
            bytes += sz;
            count += 1;
            *budget = budget.saturating_sub(1);
            files.push(Found {
                path,
                size: sz,
                group: g,
                rule: None,
                is_dir: false,
                anchors: vec![root.clone()],
            });
        }
    }
    (bytes, count)
}

/// Walk one application rule: every folder it names, and the files that match.
fn scan_rule(
    index: usize,
    rule: &AppRule,
    exclude: &[PathBuf],
    budget: &mut usize,
    files: &mut Vec<Found>,
    seen: &mut HashSet<PathBuf>,
) -> (u64, usize) {
    let mut bytes = 0u64;
    let mut count = 0usize;
    for (dir, key) in rule.targets() {
        if *budget == 0 {
            break;
        }
        if !exclude.is_empty() && under_any(&dir, exclude) {
            continue;
        }
        // The folder itself is the anchor: a rule may name a path with `*` in it,
        // so the resolved folder is what deletion is checked against.
        let anchor = dir.clone();
        let mut stack = vec![dir];
        while let Some(d) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&d) else {
                continue;
            };
            for e in entries.flatten() {
                if *budget == 0 {
                    return (bytes, count);
                }
                let path = e.path();
                let Ok(meta) = e.metadata() else { continue };
                if meta.is_dir() {
                    // Only a RECURSE key descends, and never through a link.
                    if key.recurse && !is_link(&meta) && stack.len() < 4096 {
                        stack.push(path);
                    }
                    continue;
                }
                let name = file_name_lower(&path);
                if !crate::rules::glob_match(&key.file, &name) {
                    continue;
                }
                if rule.excluded(&path) || under_any(&path, exclude) {
                    continue;
                }
                if !seen.insert(path.clone()) {
                    continue;
                }
                let sz = meta.len();
                bytes += sz;
                count += 1;
                *budget = budget.saturating_sub(1);
                files.push(Found {
                    path,
                    size: sz,
                    group: Group::AppRule,
                    rule: Some(index),
                    is_dir: false,
                    anchors: vec![anchor.clone()],
                });
            }
        }
    }
    (bytes, count)
}
/// The per-group rule for a single file: what counts as junk in that folder.
///
/// Applied during the scan and again, through [`still_wanted`], at the moment of
/// deletion, so a file replaced in between is re-judged rather than removed blindly.
fn wanted_file(g: Group, p: &Path) -> bool {
    match g {
        Group::RecycleBin => is_recycled_item(p),
        Group::Downloads => old_enough(p, WEEK) && interesting_download(p),
        Group::ThumbnailCache => is_thumb_cache(p),
        Group::Prefetch => has_extension(p, "pf"),
        Group::FontCache => has_extension(p, "dat"),
        Group::BrowserCookies => is_cookie_file(p),
        Group::ShaderCache => !has_extension(p, "txt"),
        _ => true,
    }
}

/// The files that actually hold a browser's cookies and sign-in tickets.
fn is_cookie_file(p: &Path) -> bool {
    matches!(
        file_name_lower(p).as_str(),
        "cookies" | "cookies-journal" | "cookies.sqlite" | "cookies.sqlite-wal"
    )
}

fn has_extension(p: &Path, ext: &str) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case(ext))
        .unwrap_or(false)
}
/// In `$Recycle.Bin`, `$R…` is the item the user deleted and `$I…` is its receipt
/// (original path and date). Only the `$R…` files are worth listing or counting:
/// emptying the bin takes the receipts with it, so deleting them alone frees
/// nothing and would leave `$R…` behind.
fn is_recycled_item(p: &Path) -> bool {
    file_name_lower(p).starts_with("$r")
}

fn file_name_lower(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

fn dir_size(dir: &Path, budget: &mut usize) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            if *budget == 0 {
                return total;
            }
            *budget = budget.saturating_sub(1);
            let Ok(m) = e.metadata() else { continue };
            if m.is_dir() {
                if stack.len() < 4096 {
                    stack.push(e.path());
                }
            } else {
                total += m.len();
            }
        }
    }
    total
}

/// What one scan should look at. Built-in categories, application rules, the
/// user's own folders, and the folders to leave alone.
#[derive(Default, Clone)]
pub struct Request {
    pub groups: Vec<Group>,
    /// Indices into the loaded rule list.
    pub rules: Vec<usize>,
    pub custom: Vec<PathBuf>,
    pub exclude: Vec<PathBuf>,
}

/// Scan what the request asks for. `budget` caps how many entries we look at.
///
/// Exclusions are applied while scanning, so an excluded file is never listed
/// and therefore never in a report that could be cleaned.
pub fn scan(req: &Request, rules: &[AppRule], budget: usize) -> Report {
    let exclude: Vec<PathBuf> = req.exclude.iter().map(|p| normalize(p)).collect();
    let mut files = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut per_group = Vec::new();
    let mut left = budget;
    for &g in &req.groups {
        let (b, n) = scan_group(g, &req.custom, &exclude, &mut left, &mut files, &mut seen);
        per_group.push((g, b, n));
    }
    let mut rule_bytes = 0u64;
    let mut rule_count = 0usize;
    for &i in &req.rules {
        let Some(rule) = rules.get(i) else { continue };
        let (b, n) = scan_rule(i, rule, &exclude, &mut left, &mut files, &mut seen);
        rule_bytes += b;
        rule_count += n;
    }
    if rule_count > 0 {
        per_group.push((Group::AppRule, rule_bytes, rule_count));
    }
    Report { per_group, files }
}

/// Is `p` inside, or the same as, one of the excluded folders?
fn under_any(p: &Path, exclude: &[PathBuf]) -> bool {
    let p = normalize(p);
    exclude.iter().any(|e| p.starts_with(e))
}

/// Do two paths name the same folder? Windows compares paths without case, and
/// the same folder can be typed with or without a trailing separator.
pub fn same_folder(a: &Path, b: &Path) -> bool {
    normalize(a) == normalize(b)
}

/// Is `p` inside `dir`, or `dir` itself?
pub fn inside(p: &Path, dir: &Path) -> bool {
    normalize(p).starts_with(normalize(dir))
}

/// Remove one entry, refusing anything that could reach outside the wanted folders.
///
/// A scan can be minutes old by the time the user presses the button, and inside a
/// junk folder anything may have appeared since: a junction, a symlink, a hard link
/// to a file somewhere important. The rules below keep a deletion to the tree the
/// user asked about. Returns the bytes freed, or 0 when the entry was refused.
fn remove(f: &Found) -> u64 {
    let got = if f.is_dir {
        remove_tree(&f.path)
    } else if unlink_file(&f.path) {
        f.size
    } else {
        0
    };
    if f.group == Group::RecycleBin && !f.path.exists() {
        // Delete the companion `$I…` receipt so Explorer doesn't keep a broken entry.
        if let Some(receipt) = companion_receipt(&f.path) {
            let _ = unlink_file(&receipt);
        }
    }
    got
}

/// Companion `$I…` receipt for a `$R…` item inside `$Recycle.Bin`.
fn companion_receipt(p: &Path) -> Option<PathBuf> {
    let name = p.file_name()?.to_string_lossy();
    if name.starts_with("$R") || name.starts_with("$r") {
        Some(p.with_file_name(format!("$I{}", &name[2..])))
    } else {
        None
    }
}

/// Remove a single non-directory file, clearing read-only if Windows refuses first.
fn unlink_file(p: &Path) -> bool {
    if std::fs::remove_file(p).is_ok() {
        return true;
    }
    let Ok(meta) = std::fs::symlink_metadata(p) else {
        return false;
    };
    if is_link(&meta) || !meta.permissions().readonly() {
        return false;
    }
    let mut perms = meta.permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    let _ = std::fs::set_permissions(p, perms);
    std::fs::remove_file(p).is_ok()
}

/// Delete a directory tree without following any link inside it.
fn remove_tree(root: &Path) -> u64 {
    let mut freed = 0u64;
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let Ok(meta) = std::fs::symlink_metadata(e.path()) else {
                continue;
            };
            if is_link(&meta) {
                // Unlink the link itself; never walk or delete what it points at.
                let _ = std::fs::remove_file(e.path()).or_else(|_| std::fs::remove_dir(e.path()));
                continue;
            }
            if meta.is_dir() {
                dirs.push(e.path());
                if stack.len() < 4096 {
                    stack.push(e.path());
                }
            } else {
                let sz = meta.len();
                if unlink_file(&e.path()) {
                    freed += sz;
                }
            }
        }
    }
    // Children before parents.
    for d in dirs.iter().rev() {
        let _ = std::fs::remove_dir(d);
    }
    let _ = std::fs::remove_dir(root);
    freed
}

#[cfg(windows)]
fn is_link(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    // A symlink, a junction, or any other reparse point: deleting through one of
    // these would reach a folder the user never pointed us at.
    meta.file_type().is_symlink() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_link(meta: &std::fs::Metadata) -> bool {
    meta.file_type().is_symlink()
}

/// Is this path one we are willing to delete for this group?
///
/// It must still sit inside one of the group's folders and must not be a link.
/// `allow_root` lets an entry name a folder itself, which only the groups that
/// remove a whole folder in one piece (`Windows.old`, the user's own folders) do.
#[cfg(test)]
pub fn safe_to_remove(path: &Path, group: Group) -> bool {
    let dirs: Vec<PathBuf> = roots(group).iter().map(|r| normalize(r)).collect();
    allowed(path, &dirs, false)
}

/// The cheap half of [`safe_to_remove`], for a caller that already resolved the roots.
fn allowed(path: &Path, dirs: &[PathBuf], allow_root: bool) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if is_link(&meta) {
        return false;
    }
    let p = normalize(path);
    dirs.iter()
        .any(|root| p.starts_with(root) && (p != *root || allow_root))
}

/// The rules a scan applied, re-applied at the moment of deletion: a file in
/// Downloads may have been replaced by something the user wants to keep.
fn still_wanted(f: &Found) -> bool {
    if f.rule.is_some() {
        // An application-rule entry matched its rule's file pattern during the
        // scan, and the rule list is loaded once per run, so the pattern that
        // matched still holds. The rule's own exclusions are re-checked in
        // `clean_impl`, where the rule list is available.
        return true;
    }
    if f.is_dir {
        return matches!(
            f.group,
            Group::RecycleBin | Group::WindowsOld | Group::Custom
        );
    }
    wanted_file(f.group, &f.path)
}

/// The folder roots that were scanned for this entry, normalized.
///
/// The scan records them, so the guard compares against what was actually walked
/// rather than against a list of settings that may have changed since.
fn anchors_of(f: &Found) -> Vec<PathBuf> {
    if f.anchors.is_empty() {
        roots(f.group).iter().map(|r| normalize(r)).collect()
    } else {
        f.anchors.iter().map(|r| normalize(r)).collect()
    }
}

fn is_thumb_cache(p: &Path) -> bool {
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    name.ends_with(".db") || name.contains("thumbcache") || name.contains("iconcache")
}

/// Lowercase (Windows paths are case-insensitive) and drop `.`/`..` without
/// touching the disk, so a path that only exists on paper cannot slip through.
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str().to_string_lossy().to_lowercase()),
        }
    }
    out
}

/// Do the files of this group go to the Recycle Bin rather than away for good?
///
/// Only the one group that touches things the user may still want: everything else
/// is a cache, and a cache that lands in the bin is not a benefit to anyone.
fn goes_to_recycle(group: Group) -> bool {
    matches!(group, Group::Downloads)
}

/// Move one file to the Recycle Bin. Never reports success it cannot see.
#[cfg(windows)]
pub(crate) fn recycle(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE,
        SHFILEOPSTRUCTW,
    };

    // The shell wants a list ending in two NULs.
    let mut from: Vec<u16> = path.as_os_str().encode_wide().collect();
    from.push(0);
    from.push(0);

    let mut op: SHFILEOPSTRUCTW = unsafe { std::mem::zeroed() };
    op.wFunc = FO_DELETE;
    op.pFrom = from.as_ptr();
    op.fFlags = (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI) as u16;
    unsafe { SHFileOperationW(&mut op) == 0 && op.fAnyOperationsAborted == 0 }
}

#[cfg(not(windows))]
pub(crate) fn recycle(_path: &Path) -> bool {
    false
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct CleanResult {
    pub freed: u64,
    pub done: usize,
    pub failed: usize,
    /// Items that went to the Recycle Bin; their bytes are not freed yet.
    pub recycled: usize,
    pub dry_run: bool,
}

/// Delete everything in the report the filter accepts.
///
/// The filter sees the whole entry, not just its group, so a caller can pick one
/// application rule without caring about the others. `rules` is passed again so
/// a rule's own `ExcludeKey` lines are honoured at the moment of deletion, not
/// only when the scan was made.
pub fn clean(
    report: &Report,
    rules: &[AppRule],
    want: &dyn Fn(&Found) -> bool,
    recycle_bin: bool,
) -> CleanResult {
    clean_impl(report, rules, want, recycle_bin, false)
}

/// The same pass, but it only says what it would delete.
pub fn dry_run(
    report: &Report,
    rules: &[AppRule],
    want: &dyn Fn(&Found) -> bool,
    recycle_bin: bool,
) -> CleanResult {
    clean_impl(report, rules, want, recycle_bin, true)
}

fn clean_impl(
    report: &Report,
    rules: &[AppRule],
    want: &dyn Fn(&Found) -> bool,
    recycle_bin: bool,
    dry: bool,
) -> CleanResult {
    let mut res = CleanResult {
        dry_run: dry,
        ..Default::default()
    };
    for f in &report.files {
        if !want(f) || !still_wanted(f) {
            if want(f) {
                res.failed += 1;
            }
            continue;
        }
        // A rule may have been edited between the scan and the click; re-read its
        // exclusions so a file it now protects is not removed.
        if let Some(i) = f.rule {
            if let Some(rule) = rules.get(i) {
                if rule.excluded(&f.path) {
                    res.failed += 1;
                    continue;
                }
            }
        }
        // A whole-folder entry names the folder itself, so the root is allowed.
        let root_entry = f.is_dir && matches!(f.group, Group::WindowsOld | Group::Custom);
        if !allowed(&f.path, &anchors_of(f), root_entry) {
            res.failed += 1;
            continue;
        }
        if dry {
            res.done += 1;
            res.freed += f.size;
            continue;
        }
        if !f.is_dir && goes_to_recycle(f.group) {
            if recycle(&f.path) {
                res.done += 1;
                res.recycled += 1;
            } else {
                res.failed += 1;
            }
            continue;
        }
        let got = remove(f);
        if got == 0 && f.size > 0 {
            res.failed += 1;
        } else {
            res.done += 1;
            res.freed += got;
        }
    }
    if !dry
        && recycle_bin
        && report
            .files
            .iter()
            .any(|f| f.group == Group::RecycleBin && !f.path.starts_with(std::env::temp_dir()))
    {
        empty_recycle_bin();
    }
    res
}

/// Ask Windows Shell to finish emptying the Recycle Bin and refresh its icon.
#[cfg(windows)]
fn empty_recycle_bin() {
    use windows_sys::Win32::UI::Shell::{
        SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND,
    };
    unsafe {
        SHEmptyRecycleBinW(
            std::ptr::null_mut(),
            std::ptr::null(),
            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
        );
    }
}

#[cfg(not(windows))]
fn empty_recycle_bin() {}

/// Human-readable size, e.g. "1.4 GB".
pub fn human(bytes: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", bytes, U[0])
    } else {
        format!("{:.1} {}", v, U[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_read_sensibly() {
        assert_eq!(human(0), "0 B");
        assert_eq!(human(2048), "2.0 KB");
        assert_eq!(human(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn scan_finds_and_removes_a_file() {
        let (dir, f, found) = sandbox("junk.tmp", b"0123456789", Group::UserTemp);
        assert!(f.exists());
        let rep = Report {
            per_group: vec![(Group::UserTemp, 10, 1)],
            files: vec![found],
        };
        let res = clean(&rep, &[], &|f| f.group == Group::UserTemp, false);
        assert_eq!(res.done, 1);
        assert_eq!(res.freed, 10);
        assert!(!f.exists(), "the file should be gone");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn dry_run_counts_without_deleting() {
        let (dir, f, found) = sandbox("kept.tmp", b"0123456789", Group::UserTemp);
        let rep = Report {
            per_group: vec![(Group::UserTemp, 10, 1)],
            files: vec![found],
        };
        let res = dry_run(&rep, &[], &|f| f.group == Group::UserTemp, false);
        assert!(res.dry_run);
        assert_eq!((res.done, res.freed), (1, 10));
        assert!(f.exists(), "a dry run must not touch the disk");

        let after = clean(&rep, &[], &|f| f.group == Group::UserTemp, false);
        assert_eq!(after.done, 1);
        assert!(!f.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn only_the_ticked_groups_go() {
        let (dir, f, found) = sandbox("other.tmp", b"12345", Group::UserTemp);
        let rep = Report {
            per_group: vec![(Group::UserTemp, 5, 1)],
            files: vec![found],
        };
        let res = clean(&rep, &[], &|f| f.group == Group::RecycleBin, false);
        assert_eq!(res.done, 0);
        assert!(f.exists(), "unticked groups stay");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn refuses_a_path_outside_the_group_folder() {
        // A report entry that points at something the group never scans.
        let outside = std::env::temp_dir()
            .parent()
            .unwrap()
            .join("cleaner-not-in-temp.txt");
        assert!(!safe_to_remove(&outside, Group::WindowsTemp));
        assert!(!safe_to_remove(
            Path::new("C:\\Windows\\System32"),
            Group::WindowsTemp
        ));
    }

    #[test]
    fn refuses_the_folder_itself() {
        if let Some(root) = roots(Group::UserTemp).into_iter().next() {
            assert!(!safe_to_remove(&root, Group::UserTemp));
        }
    }

    #[test]
    fn a_report_entry_outside_the_folder_is_not_deleted() {
        let (dir, f, mut found) = sandbox("guard.tmp", b"1234", Group::UserTemp);
        // Pretend a scan said this file belongs to a folder it is not in.
        found.group = Group::RecentDocs;
        let rep = Report {
            per_group: vec![(Group::RecentDocs, 4, 1)],
            files: vec![found],
        };
        let res = clean(&rep, &[], &|f| f.group == Group::RecentDocs, false);
        assert_eq!(res.failed, 1);
        assert!(f.exists(), "the guard must keep it");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn prefetch_only_takes_pf_files() {
        assert!(wanted_file(
            Group::Prefetch,
            Path::new("C:\\Windows\\Prefetch\\CHROME.EXE-1234.pf")
        ));
        assert!(!wanted_file(
            Group::Prefetch,
            Path::new("C:\\Windows\\Prefetch\\Layout.ini")
        ));
        assert!(wanted_file(
            Group::FontCache,
            Path::new("C:\\Windows\\ServiceProfiles\\LocalService\\AppData\\Local\\FontCache\\FontCache.dat")
        ));
        assert!(wanted_file(
            Group::BrowserCookies,
            Path::new("C:\\Users\\me\\AppData\\Local\\Google\\Chrome\\User Data\\Default\\Network\\Cookies")
        ));
        assert!(!wanted_file(
            Group::BrowserCookies,
            Path::new("C:\\Users\\me\\AppData\\Local\\Google\\Chrome\\User Data\\Default\\Network\\Network Persistent State")
        ));
    }

    #[test]
    fn custom_folders_are_scanned_and_exclusions_are_left_alone() {
        let base = std::env::temp_dir().join(format!("cleaner-custom-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let keep = base.join("keep");
        let hide = base.join("hide");
        std::fs::create_dir_all(&keep).unwrap();
        std::fs::create_dir_all(&hide).unwrap();
        std::fs::write(keep.join("a.tmp"), b"12345").unwrap();
        std::fs::write(hide.join("b.tmp"), b"1234567").unwrap();

        // Everything under `hide` is excluded, so only `keep` is listed (as one folder).
        let rep = scan(
            &Request {
                groups: vec![Group::Custom],
                rules: Vec::new(),
                custom: vec![base.clone()],
                exclude: vec![hide.clone()],
            },
            &[],
            10_000,
        );
        assert_eq!(rep.files.len(), 1);
        assert_eq!(rep.files[0].path, keep);
        assert!(rep.files[0].is_dir);
        assert_eq!(rep.files[0].group, Group::Custom);

        // The guard must accept this folder itself: it was one of the scanned roots.
        assert!(allowed(
            &rep.files[0].path,
            &anchors_of(&rep.files[0]),
            true
        ));

        let res = clean(&rep, &[], &|f| f.group == Group::Custom, false);
        assert_eq!(res.done, 1);
        assert!(
            !keep.exists(),
            "the custom folder's contents should be gone"
        );
        assert!(
            hide.join("b.tmp").exists(),
            "an excluded folder is never touched"
        );
        assert!(base.exists(), "the folders themselves stay");
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn a_rule_scans_and_cleans_only_what_it_names() {
        let base = std::env::temp_dir().join(format!("cleaner-rule-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let cache = base.join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("a.tmp"), b"12345").unwrap();
        std::fs::write(cache.join("keep.db"), b"1234567").unwrap();

        let ini = format!(
            "[Probe]\nDefault=True\nDetectFile={}\n\
             FileKey1={}\\cache|*.*|RECURSE\n\
             ExcludeKey1=FILE|{}\\cache|*.db\n",
            base.display(),
            base.display(),
            base.display()
        );
        let rules = crate::rules::parse(&ini);
        assert_eq!(rules.len(), 1);
        assert!(rules[0].present, "the detect folder exists");

        let req = Request {
            groups: Vec::new(),
            rules: vec![0],
            custom: Vec::new(),
            exclude: Vec::new(),
        };
        let rep = scan(&req, &rules, 1_000);
        assert_eq!(rep.files.len(), 1, "the excluded file stays out");
        assert_eq!(rep.files[0].path, cache.join("a.tmp"));
        assert_eq!(rep.files[0].rule, Some(0));
        assert_eq!(rep.files[0].group, Group::AppRule);
        assert_eq!(rep.rule_totals(0), (5, 1));

        let res = clean(&rep, &rules, &|f| f.rule == Some(0), false);
        assert_eq!(res.done, 1);
        assert!(!cache.join("a.tmp").exists());
        assert!(cache.join("keep.db").exists(), "ExcludeKey wins");
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn one_file_two_scanners_name_is_listed_once() {
        let base = std::env::temp_dir().join(format!("cleaner-dupe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let cache = base.join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("a.tmp"), b"12345").unwrap();

        let ini = format!(
            "[One]\nDetectFile={}\nFileKey1={}\\cache|*.tmp\n\
             [Two]\nDetectFile={}\nFileKey1={}\\cache|*.tmp\n",
            base.display(),
            base.display(),
            base.display(),
            base.display()
        );
        let rules = crate::rules::parse(&ini);
        assert_eq!(rules.len(), 2);

        // Two rules walk the same folder; the file is still one entry, or the
        // total and the removal would both be wrong.
        let req = Request {
            groups: Vec::new(),
            rules: vec![0, 1],
            custom: Vec::new(),
            exclude: Vec::new(),
        };
        let rep = scan(&req, &rules, 1_000);
        let count = rep
            .files
            .iter()
            .filter(|f| f.path == cache.join("a.tmp"))
            .count();
        assert_eq!(count, 1, "counted once, whichever rule saw it first");
        std::fs::remove_dir_all(&base).unwrap();
    }

    /// A throwaway directory inside %TEMP% (which is the `UserTemp` root).
    fn sandbox(name: &str, bytes: &[u8], group: Group) -> (PathBuf, PathBuf, Found) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "cleaner-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join(name);
        std::fs::write(&f, bytes).unwrap();
        let found = Found {
            path: f.clone(),
            size: bytes.len() as u64,
            group,
            is_dir: false,
            rule: None,
            anchors: Vec::new(),
        };
        (dir, f, found)
    }

    #[test]
    fn recycle_bin_lives_at_the_drive_root() {
        // The old code looked in %SystemRoot%\$Recycle.Bin, which never exists:
        // every drive has its own bin, directly under the drive root.
        #[cfg(windows)]
        {
            assert_eq!(root_of(Path::new("C:")), Path::new("C:\\"));
            assert_eq!(root_of(Path::new("C:\\Users\\me")), Path::new("C:\\"));
            let drives = fixed_drives();
            assert!(!drives.is_empty(), "at least the system drive");
            assert!(drives.iter().all(|d| d.has_root() && d.is_absolute()));
            for d in &drives {
                let bin = d.join("$Recycle.Bin");
                assert!(
                    bin.to_string_lossy()
                        .to_lowercase()
                        .ends_with(":\\$recycle.bin"),
                    "{}",
                    bin.display()
                );
            }
            // The system drive always exists, so a bin there is a root we scan.
            if Path::new("C:\\$Recycle.Bin").is_dir() {
                assert!(
                    roots(Group::RecycleBin)
                        .iter()
                        .any(|r| file_name_lower(r) == "$recycle.bin"),
                    "the per-drive bin must be a root"
                );
            }
            let win_bin = env_dir("SystemRoot").unwrap().join("$Recycle.Bin");
            assert!(
                !roots(Group::RecycleBin).contains(&win_bin),
                "the bin is not under Windows"
            );
        }
    }

    #[test]
    fn only_the_deleted_items_of_the_bin_count() {
        // `$I…` is the receipt: deleting it frees nothing and strands its `$R…`.
        assert!(is_recycled_item(Path::new(
            "C:\\$Recycle.Bin\\S-1-5\\$RAB12CD.txt"
        )));
        assert!(is_recycled_item(Path::new(
            "C:\\$Recycle.Bin\\S-1-5\\$rab12cd"
        )));
        assert!(!is_recycled_item(Path::new(
            "C:\\$Recycle.Bin\\S-1-5\\$IAB12CD.txt"
        )));
        assert!(!is_recycled_item(Path::new(
            "C:\\$Recycle.Bin\\S-1-5\\desktop.ini"
        )));
    }

    #[test]
    fn a_scanned_recycled_file_is_listed_and_removable() {
        // A fake bin shaped like the real one, scanned as a RecycleBin root would be.
        let dir = sandbox_bin();
        let sid = dir.join("S-1-5-21-1-2-3-1001");
        std::fs::create_dir_all(&sid).unwrap();
        let item = sid.join("$RABCDEF.txt");
        let receipt = sid.join("$IABCDEF.txt");
        std::fs::write(&item, b"0123456789").unwrap();
        std::fs::write(&receipt, b"receipt").unwrap();

        let listed = walk_like_scan(&dir);
        assert_eq!(listed.len(), 1, "one item, the receipt skipped");
        assert_eq!(listed[0].path, item);
        assert_eq!(listed[0].size, 10);
        assert!(!listed[0].is_dir);

        assert_eq!(remove(&listed[0]), 10);
        assert!(!item.exists(), "the item should be gone");
        assert!(!receipt.exists(), "the companion receipt is also deleted");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_recycled_folder_is_one_entry_of_its_own_size() {
        let dir = sandbox_bin();
        let sid = dir.join("S-1-5-21-1-2-3-1001");
        let item = sid.join("$RDEFGHI");
        std::fs::create_dir_all(item.join("inner")).unwrap();
        std::fs::write(item.join("a.bin"), b"12345").unwrap();
        std::fs::write(item.join("inner").join("b.bin"), b"678").unwrap();
        std::fs::write(sid.join("$IDEFGHI"), b"receipt").unwrap();

        let listed = walk_like_scan(&dir);
        assert_eq!(listed.len(), 1, "a deleted folder is a single entry");
        assert!(listed[0].is_dir);
        assert_eq!(listed[0].size, 8);
        assert!(
            still_wanted(&listed[0]),
            "recycled folders must be removable"
        );
        assert_eq!(remove(&listed[0]), 8);
        assert!(!item.exists());
        assert!(!sid.join("$IDEFGHI").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A throwaway directory standing in for `X:\$Recycle.Bin`.
    fn sandbox_bin() -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "cleaner-bin-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// What `scan_group` reports for a bin root: its `Group::RecycleBin` branch.
    fn walk_like_scan(root: &Path) -> Vec<Found> {
        let mut budget = 10_000usize;
        let mut files = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(d) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&d) else {
                continue;
            };
            for e in entries.flatten() {
                let path = e.path();
                let Ok(meta) = e.metadata() else { continue };
                if meta.is_dir() {
                    let name = file_name_lower(&path);
                    if name.starts_with("$r") {
                        let sz = dir_size(&path, &mut budget);
                        files.push(Found {
                            path,
                            size: sz,
                            group: Group::RecycleBin,
                            is_dir: true,
                            rule: None,
                            anchors: Vec::new(),
                        });
                    } else if !name.starts_with('$') {
                        stack.push(path);
                    }
                } else if is_recycled_item(&path) {
                    files.push(Found {
                        path,
                        size: meta.len(),
                        group: Group::RecycleBin,
                        is_dir: false,
                        rule: None,
                        anchors: Vec::new(),
                    });
                }
            }
        }
        files
    }
}
