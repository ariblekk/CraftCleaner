//! A desktop cleaner: finds temporary and junk files, shows what it found and
//! removes what you tick. Written from scratch in Rust with the egui toolkit.
//!
//! Layout and behaviour follow the well-known shape of a system cleaner:
//! a sidebar with Cleaner / Registry / Tools / Options, a tabbed list of file
//! categories with checkboxes, and a big Analyze button with a size ring.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod admin;
mod app;
mod clean;
mod i18n;
mod registry;
mod rules;
mod sysinfo;
mod ui;

use app::CleanerApp;
use clean::Tab;
use eframe::egui;
use std::path::PathBuf;
use ui::Page;

#[cfg(windows)]
pub(crate) fn open_path(target: &str) -> std::io::Result<()> {
    // shell:RecycleBinFolder and .cpl/.msc names go through explorer / control.
    if target.starts_with("shell:") {
        std::process::Command::new("explorer").arg(target).spawn()?;
    } else if target.ends_with(".cpl") {
        std::process::Command::new("control").arg(target).spawn()?;
    } else if target.ends_with(".msc") {
        std::process::Command::new("mmc").arg(target).spawn()?;
    } else {
        std::process::Command::new("explorer").arg(target).spawn()?;
    }
    Ok(())
}

#[cfg(not(windows))]
pub(crate) fn open_path(target: &str) -> std::io::Result<()> {
    std::process::Command::new("xdg-open").arg(target).spawn()?;
    Ok(())
}

fn parse_shot() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--screenshot" {
            return args.next().map(PathBuf::from);
        }
        if let Some(rest) = a.strip_prefix("--screenshot=") {
            return Some(PathBuf::from(rest));
        }
    }
    None
}

/// `--page registry` etc. opens straight on a page; used when taking pictures of
/// each screen while testing. Without it the app starts on the Cleaner page.
fn parse_page() -> Page {
    let args: Vec<String> = std::env::args().skip(1).collect();
    for (i, a) in args.iter().enumerate() {
        let value = if let Some(v) = a.strip_prefix("--page=") {
            Some(v.to_string())
        } else if a == "--page" {
            args.get(i + 1).cloned()
        } else {
            None
        };
        match value.as_deref() {
            Some("registry") => return Page::Registry,
            Some("tools") => return Page::Tools,
            Some("options") => return Page::Options,
            Some("info") => return Page::Info,
            _ => {}
        }
    }
    Page::Cleaner
}

/// `--tab applications` etc. picks the cleaner tab, for the same reason: the
/// Applications tab is where the loaded program rules are drawn.
fn parse_tab() -> Option<Tab> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    for (i, a) in args.iter().enumerate() {
        let value = if let Some(v) = a.strip_prefix("--tab=") {
            Some(v.to_string())
        } else if a == "--tab" {
            args.get(i + 1).cloned()
        } else {
            None
        };
        match value.as_deref() {
            Some("windows") => return Some(Tab::Windows),
            Some("applications") => return Some(Tab::Applications),
            Some("advanced") => return Some(Tab::Advanced),
            _ => {}
        }
    }
    None
}

/// `--search <text>` pre-fills the cleaner search box. Used with `--screenshot`
/// to photograph a section that sits below the first screenful of rows.
fn parse_search() -> String {
    let args: Vec<String> = std::env::args().skip(1).collect();
    for (i, a) in args.iter().enumerate() {
        if let Some(v) = a.strip_prefix("--search=") {
            return v.to_string();
        }
        if a == "--search" {
            return args.get(i + 1).cloned().unwrap_or_default();
        }
    }
    String::new()
}

/// icon.png decoded once; used for the window icon and the sidebar logo.
pub(crate) fn icon_rgba() -> image::RgbaImage {
    image::load_from_memory(include_bytes!("../icon.png"))
        .expect("icon.png must be a valid PNG")
        .to_rgba8()
}

/// The window and taskbar icon.
fn app_icon() -> egui::IconData {
    let img = icon_rgba();
    let (width, height) = img.dimensions();
    egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    }
}

fn main() -> eframe::Result<()> {
    let shot = parse_shot();
    // `--confirm` opens the delete check once a scan has landed, so it can be
    // photographed with a real list behind it. Only with `--screenshot`: a plain run
    // must never open a prompt by itself.
    let confirm = shot.is_some() && std::env::args().any(|a| a == "--confirm");
    let start_page = parse_page();
    let start_tab = parse_tab();
    let start_search = parse_search();
    let dry_run = std::env::args().any(|a| a == "--dry-run");
    // `--no-scan` skips the start-up scan, so the Cleaner page stays in its empty
    // state. Used with `--screenshot` to photograph that state.
    let no_scan = std::env::args().any(|a| a == "--no-scan");

    // Windows Temp, the update cache and the machine-wide startup list are
    // administrator-only, so ask for rights once, at start. If the person
    // declines, carry on without them: the user-level folders still work.
    // Screenshot runs are exempt - they are automation, and a consent prompt
    // has nobody to answer it.
    if shot.is_none() && !admin::is_elevated() && admin::restart_elevated() {
        return Ok(());
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1000.0, 680.0])
        .with_min_inner_size([560.0, 420.0])
        .with_icon(app_icon())
        .with_title("Cleaner");

    if shot.is_some() {
        // Do not steal focus from whatever the person is doing.
        viewport = viewport.with_active(false);
        // On Windows also sit behind other windows; they still draw there.
        #[cfg(windows)]
        {
            viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }

    let options = eframe::NativeOptions {
        viewport,
        vsync: true,
        ..Default::default()
    };

    eframe::run_native(
        "Cleaner",
        options,
        Box::new(move |cc| {
            let mut app = CleanerApp::new(cc, shot.clone());
            app.page = start_page;
            app.dry_run = dry_run;
            app.no_start_scan = no_scan;
            if let Some(tab) = start_tab {
                app.tab = tab;
            }
            app.search = start_search;
            // `--confirm` opens the delete check once the scan has landed, so it can
            // be photographed without a click. It changes nothing on its own:
            // answering Delete still goes through the same guards, and a screenshot
            // run never cleans.
            app.open_confirm = confirm;
            Ok(Box::new(app))
        }),
    )
}
