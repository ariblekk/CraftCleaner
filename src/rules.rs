//! Application cleaning rules, read from INI files.
//!
//! The shape of the file is the one the community has used for years (the
//! `winapp2.ini` format): one section per program, a `Detect`/`DetectFile` line
//! that says when the section applies, and `FileKey`/`ExcludeKey` lines that say
//! what may go and what must stay.
//!
//! Nothing here deletes. A rule is turned into a list of folders and file
//! patterns; the cleaner does the walking, so the same guards apply to a rule
//! entry as to a built-in category.
//!
//! Reading rules written by other people means trusting files we did not write,
//! so the parser is deliberately narrow:
//!
//! - a section is skipped unless at least one `Detect`/`DetectFile` line is
//!   present and true on this machine, so no rule can apply "by accident";
//! - only `FileKey` is acted on. `RegKey`, `SpecialKey` and the rest are ignored
//!   rather than half-understood;
//! - `%VAR%` is expanded from the environment and nothing else: a path that
//!   still holds a `%` after expansion is dropped;
//! - a wildcard path is only expanded by listing real directories, never by
//!   guessing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
/// One folder to look in, and what counts as junk there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    /// Folder, after `%VAR%` expansion; may still hold `*` or `?`.
    pub dir: String,
    /// File name pattern (the part after the first `|`).
    pub file: String,
    /// Descend into subfolders.
    pub recurse: bool,
}

/// A file or folder a rule says must never be removed, whatever a `FileKey` says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Exclude {
    /// `ExcludeKey=FILE|dir|pattern`: files matching `pattern` under `dir`.
    File { dir: String, file: String },
    /// `ExcludeKey=PATH|dir`: `dir` itself and everything under it.
    Path { dir: String },
}

/// One program's rules.
#[derive(Debug, Clone)]
pub struct AppRule {
    pub name: String,
    /// Tick this program's box the first time it is seen.
    pub default: bool,
    /// True when at least one `Detect`/`DetectFile` holds on this machine.
    pub present: bool,
    /// The `Detect`/`DetectFile` lines, kept so the check can be repeated.
    detect: Vec<Detect>,
    pub keys: Vec<Key>,
    pub excludes: Vec<Exclude>,
    /// The `Warning=` line, shown as the row's note when there is one.
    pub warning: Option<String>,
}

impl AppRule {
    /// Folders to walk, with the patterns to look for. Only folders that exist.
    pub fn targets(&self) -> Vec<(PathBuf, Key)> {
        let mut out = Vec::new();
        for k in &self.keys {
            for dir in expand_dirs(&k.dir) {
                out.push((dir, k.clone()));
            }
        }
        // The same folder and pattern twice (`path\*` meeting `cache` for example)
        // would be walked twice for nothing. Two keys in one folder that look for
        // different things are both kept.
        out.sort_by(|a, b| (&a.0, &a.1.file, a.1.recurse).cmp(&(&b.0, &b.1.file, b.1.recurse)));
        out.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
        out
    }

    /// Does this rule say to leave this file alone?
    ///
    /// A literal folder is compared as text, not by looking for it on disk: an
    /// excluded folder must still protect its files if it was created after the
    /// scan, or if the user cannot read it.
    pub fn excluded(&self, path: &Path) -> bool {
        let p = normalize(path);
        let name = file_name(path);
        for ex in &self.excludes {
            match ex {
                Exclude::Path { dir } => {
                    if let Some(lit) = literal_dir(dir) {
                        if p.starts_with(normalize(&lit)) {
                            return true;
                        }
                    } else if expand_dirs(dir)
                        .iter()
                        .any(|root| p.starts_with(normalize(root)))
                    {
                        return true;
                    }
                }
                Exclude::File { dir, file } => {
                    if !glob_match(file, &name) {
                        continue;
                    }
                    if let Some(lit) = literal_dir(dir) {
                        if p.starts_with(normalize(&lit)) {
                            return true;
                        }
                    } else if expand_dirs(dir)
                        .iter()
                        .any(|root| p.starts_with(normalize(root)))
                    {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// The built-in list, as text, so a caller can count it without a second copy.
pub const BUILTIN: &str = include_str!("../rules/builtin.ini");

/// Read every rule file that exists, in order. Later files win on a repeated
/// section name, so a user's own copy can correct a built-in one.
///
/// The second value reports each extra file that was read and how many sections it
/// held, so the Options page can show whether a winapp2.ini was picked up.
pub fn load(extra: &[PathBuf]) -> (Vec<AppRule>, Vec<(PathBuf, usize)>) {
    let mut rules: Vec<AppRule> = Vec::new();
    let mut used = Vec::new();
    let mut push = |text: &str| {
        for r in parse(text) {
            if let Some(slot) = rules.iter_mut().find(|x| x.name == r.name) {
                *slot = r;
            } else {
                rules.push(r);
            }
        }
    };
    push(BUILTIN);
    for p in extra {
        if let Ok(text) = std::fs::read_to_string(p) {
            used.push((p.clone(), parse(&text).len()));
            push(&text);
        }
    }
    rules.sort_by_key(|a| a.name.to_lowercase());
    (rules, used)
}

/// The rule files to look for beyond the built-in one: next to the program, and
/// in the user's own config folder.
pub fn extra_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join("winapp2.ini"));
        }
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        out.push(PathBuf::from(appdata).join("BCleaner\\winapp2.ini"));
    }
    out
}

/// Parse an INI file into rules, dropping anything incomplete.
pub fn parse(text: &str) -> Vec<AppRule> {
    let mut out = Vec::new();
    let mut cur: Option<AppRule> = None;

    let finish = |cur: Option<AppRule>, out: &mut Vec<AppRule>| {
        if let Some(mut r) = cur {
            // A rule with nothing to detect it by could apply to machines it was
            // never written for; a rule with no file keys has nothing to do.
            if !r.detect.is_empty() && !r.keys.is_empty() {
                r.present = detects_now(&r);
                out.push(r);
            }
        }
    };

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            finish(cur.take(), &mut out);
            cur = Some(AppRule {
                name: name.trim().to_string(),
                default: false,
                present: false,
                detect: Vec::new(),
                keys: Vec::new(),
                excludes: Vec::new(),
                warning: None,
            });
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        let Some(rule) = cur.as_mut() else { continue };
        match key.to_ascii_uppercase().as_str() {
            "DEFAULT" => rule.default = value.eq_ignore_ascii_case("true"),
            "WARNING" => rule.warning = Some(value.to_string()),
            "DETECTFILE" => {
                if !value.is_empty() {
                    rule.detect.push(Detect::File(value.to_string()));
                }
            }
            "DETECT" => {
                if !value.is_empty() {
                    rule.detect.push(Detect::Reg(value.to_string()));
                }
            }
            k if k.starts_with("FILEKEY") => {
                if let Some(k) = parse_file_key(value) {
                    rule.keys.push(k);
                }
            }
            k if k.starts_with("EXCLUDEKEY") => {
                if let Some(e) = parse_exclude_key(value) {
                    rule.excludes.push(e);
                }
            }
            // `Section`, `LangSecRef`, `RegKey`, `SpecialKey`, `DetectOS` and the
            // rest are read and left alone on purpose.
            _ => {}
        }
    }
    finish(cur, &mut out);
    out
}

/// `%Path%|pattern|RECURSE`, or `%Path%\*.log|RECURSE` when the pattern is folded
/// into the path. Returns `None` for the plain `Key=Value` shape we do not model.
fn parse_file_key(value: &str) -> Option<Key> {
    let parts: Vec<&str> = value.split('|').map(|s| s.trim()).collect();
    let mut dir = (*parts.first()?).to_string();
    if dir.is_empty() {
        return None;
    }
    let mut flags: Vec<&str> = parts.iter().skip(2).copied().collect();
    let mut file = parts.get(1).copied().unwrap_or("").to_string();
    if file.is_empty() {
        // The pattern may sit on the last path segment instead.
        if let Some((head, tail)) = dir.rsplit_once(['\\', '/']) {
            if tail.contains('*') || tail.contains('?') {
                file = tail.to_string();
                dir = head.to_string();
            }
        }
    }
    if file.is_empty() {
        file = "*".to_string();
    } else if file.contains('|') {
        return None;
    }
    if let Some(rest) = parts.get(2) {
        if rest.contains('|') || (!rest.is_empty() && !rest.eq_ignore_ascii_case("RECURSE")) {
            flags = vec![*rest];
        }
    }
    let recurse = flags
        .iter()
        .any(|f| f.trim().eq_ignore_ascii_case("RECURSE"));
    Some(Key { dir, file, recurse })
}

/// `FILE|dir|pattern` or `PATH|dir`.
fn parse_exclude_key(value: &str) -> Option<Exclude> {
    let parts: Vec<&str> = value.split('|').map(|s| s.trim()).collect();
    let kind = parts.first()?.to_ascii_uppercase();
    let dir = (*parts.get(1)?).to_string();
    if dir.is_empty() {
        return None;
    }
    match kind.as_str() {
        "FILE" => {
            let file = parts.get(2).copied().unwrap_or("*").to_string();
            Some(Exclude::File {
                dir,
                file: if file.is_empty() { "*".into() } else { file },
            })
        }
        "PATH" => Some(Exclude::Path { dir }),
        // A registry exclusion says nothing about the file system.
        _ => None,
    }
}

/// Is any of a rule's `Detect`/`DetectFile` lines true here?
fn detects_now(rule: &AppRule) -> bool {
    rule.detect.iter().any(|d| match d {
        Detect::File(p) => !expand_dirs(p).is_empty(),
        Detect::Reg(k) => reg_key_exists(k),
    })
}

#[derive(Debug, Clone)]
enum Detect {
    File(String),
    Reg(String),
}

#[cfg(windows)]
fn reg_key_exists(key: &str) -> bool {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;
    let Some((hive, sub)) = key.split_once('\\') else {
        return false;
    };
    let root = match hive.to_ascii_uppercase().as_str() {
        "HKCU" | "HKEY_CURRENT_USER" => RegKey::predef(HKEY_CURRENT_USER),
        "HKLM" | "HKEY_LOCAL_MACHINE" => RegKey::predef(HKEY_LOCAL_MACHINE),
        _ => return false,
    };
    root.open_subkey(sub).is_ok()
}

#[cfg(not(windows))]
fn reg_key_exists(_key: &str) -> bool {
    false
}

/// Expand `%VAR%` in a string, then any wildcard folders, into real directories.
///
/// Only folders are returned, and only ones that exist: a rule whose program is
/// not installed simply has no targets.
pub fn expand_dirs(pattern: &str) -> Vec<PathBuf> {
    let Some(text) = expand_env(pattern) else {
        return Vec::new();
    };
    let mut parts = split_components(&text);
    if parts.is_empty() {
        return Vec::new();
    }
    // Where the walk starts: a drive root, a UNC share, or the current folder.
    let base = if text.starts_with("\\\\") || text.starts_with("//") {
        if parts.len() < 2 {
            return Vec::new();
        }
        let head = PathBuf::from(format!("\\\\{}\\{}", parts[0], parts[1]));
        parts.drain(0..2);
        head
    } else if parts[0].len() == 2 && parts[0].ends_with(':') {
        let root = PathBuf::from(format!("{}\\", parts[0]));
        parts.remove(0);
        root
    } else if text.starts_with('\\') || text.starts_with('/') {
        PathBuf::from("\\")
    } else {
        // A relative pattern: rare in a rule file, but easy to honour.
        let Ok(dir) = std::env::current_dir() else {
            return Vec::new();
        };
        dir
    };
    let mut cur = vec![base];
    for part in &parts {
        if cur.len() > MAX_DIRS {
            return Vec::new();
        }
        let mut next = Vec::new();
        if part.contains('*') || part.contains('?') {
            for dir in &cur {
                let Ok(entries) = std::fs::read_dir(dir) else {
                    continue;
                };
                for e in entries.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    if glob_match(part, &name) && e.path().is_dir() {
                        next.push(e.path());
                    }
                }
            }
        } else {
            for dir in &cur {
                let p = dir.join(part);
                if p.is_dir() {
                    next.push(p);
                }
            }
        }
        if next.is_empty() {
            return Vec::new();
        }
        cur = next;
    }
    cur.retain(|p| p.is_dir());
    cur.sort();
    cur.dedup();
    cur
}

/// A generous cap on how many folders one wildcard is allowed to fan out to.
const MAX_DIRS: usize = 4096;

/// The folder a pattern names, when it names one exactly and does not fan out.
///
/// Used for exclusions: an excluded folder has to protect its contents even if it
/// does not exist yet, so this does not touch the disk. `None` when the pattern
/// has a wildcard, which then has to be resolved against real folders.
fn literal_dir(pattern: &str) -> Option<PathBuf> {
    let text = expand_env(pattern)?;
    if text.is_empty() || text.contains('*') || text.contains('?') {
        return None;
    }
    Some(PathBuf::from(text))
}

fn split_components(text: &str) -> Vec<String> {
    text.split(['\\', '/'])
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Replace `%NAME%` with the environment value. `None` if a name is unknown, so
/// a rule cannot end up pointing at a literal `%VAR%` folder.
fn expand_env(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            // A lone `%` is not a variable.
            out.push_str(&rest[start..]);
            return Some(out);
        };
        let name = &after[..end];
        let value = env_value(name)?;
        out.push_str(&value);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// Environment lookup that does not care about case, the way Windows does not.
fn env_value(name: &str) -> Option<String> {
    static VARS: OnceLock<HashMap<String, String>> = OnceLock::new();
    let vars = VARS.get_or_init(|| {
        std::env::vars()
            .map(|(k, v)| (k.to_lowercase(), v))
            .collect()
    });
    vars.get(&name.to_lowercase()).cloned()
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// `*` and `?`, without case, the way a Windows file name pattern behaves.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let s: Vec<char> = name.to_lowercase().chars().collect();
    let (mut pi, mut si) = (0usize, 0usize);
    let (mut star, mut mark) = (None::<usize>, 0usize);
    while si < s.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = si;
            pi += 1;
        } else if let Some(sp) = star {
            pi = sp + 1;
            mark += 1;
            si = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

fn normalize(p: &Path) -> PathBuf {
    PathBuf::from(p.to_string_lossy().to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs_behave_like_file_patterns() {
        assert!(glob_match("*", "anything.log"));
        assert!(glob_match("*.log", "a.log"));
        assert!(glob_match("*.log", "A.LOG"), "names compare without case");
        assert!(!glob_match("*.log", "a.txt"));
        assert!(glob_match("cache?", "cache1"));
        assert!(glob_match("a*b*c", "axxbyyc"));
        assert!(!glob_match("a*b*c", "axxbyy"));
        assert!(glob_match("*cache*", "MyCache2"));
    }

    #[test]
    fn file_key_reads_both_shapes() {
        let k = parse_file_key("%AppData%\\Foo\\Cache|*.*|RECURSE").unwrap();
        assert_eq!(k.dir, "%AppData%\\Foo\\Cache");
        assert_eq!(k.file, "*.*");
        assert!(k.recurse);

        let k = parse_file_key("%AppData%\\Foo\\*.log").unwrap();
        assert_eq!(k.dir, "%AppData%\\Foo");
        assert_eq!(k.file, "*.log");
        assert!(!k.recurse);

        // A pattern-less key means the folder's own files.
        let k = parse_file_key("%Temp%\\Foo").unwrap();
        assert_eq!(k.file, "*");
    }

    #[test]
    fn a_section_without_a_detect_line_is_ignored() {
        let text = "[Risky]\nFileKey1=%Temp%|*.*|RECURSE\n";
        assert!(parse(text).is_empty(), "no detect, no rule");

        let text = "[Fine]\nDetectFile=%Temp%\nFileKey1=%Temp%|*.*\n";
        assert_eq!(parse(text).len(), 1);
    }

    #[test]
    fn detect_file_decides_whether_a_rule_is_present() {
        let text = "[Here]\nDetectFile=%TEMP%\nFileKey1=%Temp%|*.*\n\
                    [Elsewhere]\nDetectFile=%TEMP%\\definitely-not-a-folder-xyz\nFileKey1=%Temp%|*.*\n";
        let rules = parse(text);
        let here = rules.iter().find(|r| r.name == "Here").unwrap();
        let there = rules.iter().find(|r| r.name == "Elsewhere").unwrap();
        assert!(here.present, "%TEMP% exists");
        assert!(!there.present);
    }

    #[test]
    fn an_unknown_variable_drops_the_path() {
        assert!(expand_env("%AppData%\\X").is_some());
        assert!(expand_env("%NoSuchVariableAtAll%\\X").is_none());
        assert_eq!(expand_dirs("%NoSuchVariableAtAll%\\X").len(), 0);
    }

    #[test]
    fn excludes_win_over_the_file_keys() {
        let text =
            "[App]\nDetectFile=%TEMP%\nFileKey1=%Temp%|*.*\nExcludeKey1=FILE|%Temp%|keep*.db\n"
                .to_string();
        let rules = parse(&text);
        let r = rules.iter().find(|r| r.name == "App").unwrap();
        let root = std::env::temp_dir();
        assert!(r.excluded(&root.join("keepme.db")));
        assert!(r.excluded(&root.join("KEEP.db")), "case does not matter");
        assert!(!r.excluded(&root.join("other.db")));
    }

    #[test]
    fn an_excluded_folder_is_never_a_target() {
        let text = "[App]\nDetectFile=%TEMP%\nFileKey1=%Temp%|*.*\n\
                    ExcludeKey1=PATH|%Temp%\\keepme\n";
        let rules = parse(text);
        let r = rules.iter().find(|r| r.name == "App").unwrap();
        let inside = std::env::temp_dir().join("keepme").join("x.tmp");
        assert!(r.excluded(&inside));
    }

    #[test]
    fn two_keys_in_one_folder_look_for_both_patterns() {
        let text = "[App]\nDetectFile=%TEMP%\n\
                    FileKey1=%Temp%|*.tmp\nFileKey2=%Temp%|*.log\n";
        let rules = parse(text);
        let r = rules.iter().find(|r| r.name == "App").unwrap();
        let tmp = r.targets().into_iter().any(|(_, k)| k.file == "*.tmp");
        let log = r.targets().into_iter().any(|(_, k)| k.file == "*.log");
        assert!(tmp && log, "neither pattern may swallow the other");
    }

    #[test]
    fn the_built_in_file_parses_and_is_sane() {
        let rules = parse(include_str!("../rules/builtin.ini"));
        assert!(rules.len() >= 5, "the built-in list should have entries");
        for r in &rules {
            assert!(!r.name.is_empty());
            assert!(!r.keys.is_empty(), "{} has no file keys", r.name);
            for k in &r.keys {
                assert!(!k.dir.is_empty(), "{} has an empty path", r.name);
                assert!(!k.dir.contains(".."), "{} climbs out of its folder", r.name);
            }
        }
    }

    #[test]
    fn a_file_that_was_read_is_reported_with_its_section_count() {
        let dir = std::env::temp_dir().join(format!("cleaner-rules-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mine = dir.join("winapp2.ini");
        std::fs::write(&mine, "[Mine]\nDetectFile=%TEMP%\nFileKey1=%Temp%|*.mine\n").unwrap();
        let missing = dir.join("nope.ini");

        let (rules, read) = load(&[mine.clone(), missing]);
        assert_eq!(read.len(), 1, "only the file that exists is reported");
        assert_eq!(read[0].0, mine);
        assert_eq!(read[0].1, 1, "one section in it");
        assert!(rules.iter().any(|r| r.name == "Mine"), "and it was loaded");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
