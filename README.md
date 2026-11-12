# Cleaner

A desktop cleaner for Windows, written in Rust with egui. It finds temporary and
junk files, shows you exactly what it found, and removes only what you tick.

## What it does

**Cleaner** — three tabs (Windows, Applications, Advanced) listing the places junk
collects, each with a checkbox, a short note and, after an analysis, the size and
file count found. Every row the scan can reach stays listed whatever its size, so a
row never disappears under the click that ticked it. A search box narrows the rows
on the tab in view, and **Select all** / **Select none** tick or untick that whole
tab in one click:

| Category | Looks in |
| --- | --- |
| Temporary files (system) | `%SystemRoot%\Temp` |
| Temporary files (my account) | `%TEMP%` |
| Recycle Bin | `X:\$Recycle.Bin` on every fixed drive (`$R…` items only; `$I…` receipts are never deleted on their own) |
| Thumbnail cache | `…\AppData\Local\Microsoft\Windows\Explorer` (cache files only) |
| Windows prefetch data | `%SystemRoot%\Prefetch` (`.pf` files) |
| Font cache | `…\FontCache` under Windows and the account (`.dat` files) |
| DirectX shader cache | `%LOCALAPPDATA%\D3DSCache`, plus the NVIDIA and AMD caches |
| Windows Update leftovers | `%SystemRoot%\SoftwareDistribution\Download` |
| Delivery Optimization files | `%SystemRoot%\SoftwareDistribution\DeliveryOptimization` |
| Error reports and crash dumps | `Minidump`, `LiveKernelReports`, `CrashDumps`, WER |
| Browsers: internet cache | Chrome, Edge, Brave, Vivaldi, Firefox profile caches, `INetCache` |
| Browsers: cookies and sign-ins | the `Network` folder of each Chromium browser and every Firefox profile (cookie files only) |
| App caches | Store cache, and the Teams, Discord and Slack caches |
| Recent documents list | `…\AppData\Roaming\Microsoft\Windows\Recent` |
| Windows log files | `%SystemRoot%\Logs`, `%SystemRoot%\Panther` |
| Memory dumps | `%SystemRoot%\MEMORY.DMP` |
| Downloaded installers (over a week old) | `%USERPROFILE%\Downloads`, installer/archive types only |
| Previous Windows installation | `Windows.old`, `$WINDOWS.~BT`, `$WINDOWS.~WS` on the system drive, each listed as one folder |
| My own folders | whatever is added on the Options page |
| Programs found on this PC | one row per matching application rule (see below) |

Cookies, the previous Windows installation and the user's own folders start unticked: they are
the three that lose something the person may want back.

Under the built-in categories, the Applications tab also lists the programs found on
this PC, one row each, from the rule files described below.

The Cleaner page has two faces. At start it is an empty state: a short explanation and
one **Scan now** button, nothing else. The button starts a scan on a background thread
and the page turns into a spinner that says what is happening. When the scan finishes
the page becomes the list: the tabs, the search box and **Select all / Select none**
stay at the top, only the rows scroll under them, and a bar stuck to the bottom shows
how much is ticked, **Run Cleaner** and **Scan again**. Closing or losing the report is
not a state: a fresh start shows the empty state again, and **Scan again** returns to
the list once the new scan lands. **Run Cleaner** removes what is ticked, off the UI
thread, after a confirmation that counts what is about to go. Files that are locked or
protected are reported rather than silently kept.

**Registry** — three read-and-check passes: shortcuts in the Start menu and on the
Desktop pointing at files that no longer exist, programs told to start with Windows
whose program is gone, and installed programs whose folder is missing. Shortcuts are
read from the path stored in the `.lnk` itself, not from whatever text looks like a
path, so an icon location is never mistaken for a target. Broken shortcuts go to the
Recycle Bin, broken startup entries are deleted from the hive they came from
(`HKCU` or `HKLM`; the machine-wide one needs administrator rights). Uninstall
entries are listed read-only, with the key and path, because deleting them loses the
repair and uninstall commands.

**Tools** — buttons that open the uninstall list, the Startup folder, the Recycle
Bin, the temporary folder, Downloads and the services list, plus a drive-space bar.

**Options** — ask before deleting, analyze on start, the language (English or
Indonesian), which categories are ticked, and two folder lists: folders to empty
(the "My own folders" category) and folders never to touch. A path is typed or pasted
in — no file dialog, so nothing new to install — and checked for being a real folder,
not already listed, and not inside an excluded one. Exclusions win over everything:
an excluded file is never listed during a scan, so no report can ever contain it.
The sidebar can be folded down to its icons with the switch at its top right (a caret
points the other way to unfold it); the version sits at its foot. The choice is
remembered.

Every other page scrolls as a whole, and the layout reflows for a small window: the tool
cards widen to the row, the system-information rows stack, and the version chip and disk
figure step aside rather than overlap. On the Cleaner page only the list scrolls: the tabs
and search stay put, the footer stacks its two lines above its buttons when the window is
narrow, and every button is measured from its own label, so a longer word in one language
cannot spill out of its box. List badges are measured from their own text and the label
and note beside them are cut with an ellipsis before the badge.

**System Information** — computer name, Windows build, processor, memory (used · free,
where free counts what Windows can hand back from its caches, not just `total - used`)
and disk use.

## Application rules

Program-specific cleaning comes from INI files in the shape the community has used
for years (`winapp2.ini`). Three are read, in this order, and a later file replaces an
earlier section with the same name:

1. the built-in list, compiled into the program (`rules/builtin.ini`: Chrome, Edge,
   Brave, Vivaldi, Firefox, Discord, Teams, Slack, VS Code, Spotify, Steam, Zoom,
   Adobe Acrobat, VLC, Notepad++);
2. `winapp2.ini` next to the program;
3. `%APPDATA%\BCleaner\winapp2.ini`.

A section looks like this:

```ini
[Microsoft Teams]
DetectFile=%AppData%\Microsoft\Teams
Default=True
FileKey1=%AppData%\Microsoft\Teams\Cache|*.*|RECURSE
ExcludeKey1=PATH|%AppData%\Microsoft\Teams\IndexedDB
```

- `DetectFile=` and `Detect=` (`HKCU\...` or `HKLM\...`) say when the section applies.
  A section without either is ignored, and a section whose check fails does not appear.
  Either one being true is enough, and what they point at decides whether a program
  counts as installed: a folder that survives an uninstall (`%LocalAppData%\Adobe`, or
  `%AppData%\Code` for an editor whose data outlives it) makes a rule look present when
  the program is gone. Point detection at a file only that program writes.
- `FileKeyN=%dir%|pattern|RECURSE` is what may go. The pattern is the part after the
  first `|`; with no pattern, the folder's own files are taken. `RECURSE` descends into
  subfolders. `*` in a folder path is expanded by listing real folders only.
- `ExcludeKeyN=FILE|%dir%|pattern` and `ExcludeKeyN=PATH|%dir%` win over every
  `FileKey`, at scan time and again at the moment of deletion.
- `Default=True` starts the program's box ticked; without it the row starts unticked.
- `Warning=` is shown as the row's note. Without one, a row that found something says
  "Safe to remove", and a row that found nothing says the program was not found and only
  leftover folders matched — the honest reading of a rule that is detected but empty.
- `RegKey`, `SpecialKey`, `DetectOS` and the rest are read and left alone rather than
  half-understood. Only file keys are acted on.

Ticks are saved by section name, so editing a file or adding a new program does not
shift which programs are ticked; a newly seen program falls back to its `Default=`.

Whether a winapp2.ini was picked up is shown in two places: a source chip at the right
of the "Programs found on this PC" heading (built-in rules only, or how many files were
loaded), and a **Application rule files** card on the Options page listing the built-in
section count and each file that was read, with its path and section count.

## Running it

```
cargo run                        # normal window
cargo run -- --dry-run           # say what would go, delete nothing
cargo run -- --screenshot shots/now.png
```

A normal launch asks Windows for administrator rights first (the UAC prompt),
because Windows Temp, the update cache and the machine-wide startup list are
administrator-only. Decline it and the app still runs, with the user-level folders
and a "Run as administrator" button in the title bar.

With `--screenshot <file.png>` the app opens without taking focus (and, on Windows,
behind other windows), draws about 120 frames, writes the window as a PNG and exits.
`--screenshot` skips the consent prompt, since automation has nobody to answer it.
`--page registry|tools|options|info` opens straight on that page, and
`--tab windows|applications|advanced` picks the cleaner tab. `--search <text>`
pre-fills the search box, which is how the shots in `shots/` photograph a section
that sits below the first screenful of rows. `--no-scan` skips the start-up scan for
that run only, leaving the Cleaner page in its empty state; it does not change the saved
"Analyze on start" preference. `--confirm` (only honoured together with `--screenshot`)
opens the delete check straight away so it can be photographed; it never deletes by
itself.

## Safety

Cleaning deletes, so the deletion path is deliberately narrow:

- Every entry is re-checked at the moment it is removed: it must still sit inside one
  of the folders its category scans, and it must not be a symlink, junction or other
  reparse point. A path that changed since the scan is refused, not followed. Each
  entry carries the roots it was found under, so the check uses what was actually
  scanned rather than the settings, which may have changed in between.
- Directory trees are emptied by hand rather than with `remove_dir_all`, so a link
  planted inside a junk folder cannot redirect the delete somewhere else.
- Downloaded installers go to the Recycle Bin rather than away for good.
- A rule file is read as a description of what may go, never as permission to
  speculate: a section without a `Detect`/`DetectFile` line is dropped, an unknown
  `%VAR%` drops the whole path instead of matching a literal `%VAR%` folder, and a
  section that cannot be detected on this machine gets no targets at all.
- `--dry-run` runs the whole pass, counts what it would free, and touches nothing.
- `%SystemRoot%\Temp` and the Windows Update cache need administrator rights. The app
  asks for them when it starts instead of failing quietly; if the prompt is declined,
  the button in the title bar can ask again.

## Layout

| File | What is in it |
| --- | --- |
| `src/main.rs` | entry point, arguments, window icon, `open_path` |
| `src/app.rs` | application state, scanning, cleaning, saved settings |
| `src/ui.rs` | palette, sidebar, the five pages |
| `src/clean.rs` | finding junk, the removal rules, the tests |
| `src/rules.rs` | the application-rule engine: INI parsing, detection, wildcards |
| `rules/builtin.ini` | the built-in application rules |
| `shots/` | screenshots taken with `--screenshot` |
| `src/registry.rs` | broken shortcuts, startup entries, installed programs |
| `src/sysinfo.rs` | computer name, memory, disks |
| `src/admin.rs` | is this process elevated, restart with a consent prompt |
| `src/i18n.rs` | every word the interface says, in English and Indonesian |

## Notes

- Text is set in Archivo; sidebar icons come from Phosphor (`egui-phosphor`).
- Motion comes from `egui_animation`: folding the sidebar, hover colours, the scan
  spinner, the page fade and the sweeping progress bar all ease instead of snapping. The
  sidebar fold is one value (0..1) that drives the panel width and every part that fades,
  so nothing jumps.
- `icon.png` is the window/taskbar icon (embedded at runtime); `icon.ico`, built from
  it, becomes the `.exe` icon in Explorer via
  `build.rs` and `winresource`. The sidebar carries no brand block: the window title
  says which app this is, so the rail holds the fold switch, the pages and the version.
- Which categories and application rules are ticked, "ask before deleting", "analyze
  on start", the language and whether the sidebar is folded are saved between runs
  (`eframe` persistence, in the platform's config folder). An older settings file
  without the newer fields still loads, taking the defaults for them.
- Windows-first. On other systems the scan finds nothing rather than misbehaving.
- The scan is capped at 60,000 entries per run, so a huge folder cannot hang the app.
- `cargo test` covers size formatting, the removal guards, dry runs, colour mixing,
  a real find-and-delete pass, the per-group file rules, a scan that must leave an
  excluded folder alone, the INI parser and its detection, and a rule scan that
  removes what it names while an `ExcludeKey` file stays.
