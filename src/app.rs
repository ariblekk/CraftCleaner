//! The state of the application and everything it does: scanning, cleaning, saving.

use crate::i18n::Lang;
use crate::ui::{
    gindex, page_background, Page, ACCENT, ACCENT2, BG, CARD, CARD_HOVER, CARD_SOFT, LINE, MUTED,
    NAV, TEXT,
};
use crate::{admin, clean, registry, rules, sysinfo};
use clean::{Group, Report, Tab};
use eframe::egui;
use egui::{
    Align, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Layout, RichText, Sense,
    Stroke, Vec2,
};
use std::path::PathBuf;
use std::sync::{mpsc::Receiver, Arc};
use std::time::Instant;

/// What a clean run hands back: the report it worked from, the selection it was
/// asked to take, and what happened.
type CleanDone = (Report, Vec<Group>, Vec<usize>, clean::CleanResult);

pub(crate) struct CleanerApp {
    pub(crate) page: Page,
    pub(crate) tab: Tab,
    pub(crate) sel: [bool; Group::COUNT],
    /// What is typed in the search box above the category list.
    pub(crate) search: String,
    /// Folders the user added to empty, and folders to leave alone. Both are saved
    /// between runs; the scan reads them.
    pub(crate) custom: Vec<PathBuf>,
    pub(crate) exclude: Vec<PathBuf>,
    /// What is typed into the "add a folder" boxes on the Options page.
    pub(crate) new_custom: String,
    pub(crate) new_exclude: String,
    /// Application rules loaded at start: the built-in list plus any winapp2.ini.
    pub(crate) rules: Arc<Vec<rules::AppRule>>,
    /// The winapp2.ini files that were found and read, and how many sections each
    /// held, in load order.
    pub(crate) rule_files: Vec<(PathBuf, usize)>,
    /// How many sections the built-in list holds, before any file could replace one.
    pub(crate) builtin_rules: usize,
    /// One tick per rule, aligned with `rules`.
    pub(crate) rule_sel: Vec<bool>,
    /// Whether this process already has administrator rights (checked once).
    pub(crate) elevated: bool,
    /// Started with `--dry-run`: say what would go, touch nothing.
    pub(crate) dry_run: bool,

    pub(crate) report: Option<Report>,
    pub(crate) scanning: bool,
    pub(crate) scan_rx: Option<Receiver<Report>>,
    pub(crate) scan_started: Option<Instant>,
    pub(crate) last_scan: Option<Instant>,

    pub(crate) status: String,
    pub(crate) status_is_error: bool,
    /// When the status last changed, so the line can fade in instead of snapping.
    pub(crate) status_at: Option<Instant>,
    /// When the current page was opened, so its content can fade in.
    pub(crate) page_at: Option<Instant>,
    pub(crate) shown_page: Option<Page>,
    pub(crate) cleaning: bool,
    pub(crate) clean_rx: Option<Receiver<CleanDone>>,
    pub(crate) confirm_clean: bool,
    pub(crate) pending_clean: Option<(Vec<Group>, Vec<usize>)>,
    pub(crate) ask_before: bool,
    pub(crate) auto_analyze: bool,
    pub(crate) analyzed_once: bool,
    /// Set by `--no-scan` for one run, so a picture of the empty state can be taken
    /// without that choice being saved as the user's own preference.
    pub(crate) no_start_scan: bool,
    /// Set by `--confirm`: once the scan lands, open the delete check over the real
    /// rows, so the dialog can be photographed.
    pub(crate) open_confirm: bool,
    /// Interface language, and whether the sidebar is folded down to its icons.
    pub(crate) lang: Lang,
    pub(crate) collapsed: bool,

    pub(crate) reg_checks: [bool; 3],
    pub(crate) reg_issues: Vec<registry::Issue>,
    pub(crate) reg_sel: Vec<bool>,
    pub(crate) reg_running: bool,
    pub(crate) reg_rx: Option<Receiver<Vec<registry::Issue>>>,
    pub(crate) reg_started: Option<Instant>,
    pub(crate) reg_scanned: bool,
    pub(crate) reg_status: String,
    pub(crate) last_reg_scan: Option<Instant>,

    pub(crate) sys: sysinfo::SysInfo,

    pub(crate) shot: Option<PathBuf>,
    pub(crate) shot_started: Option<Instant>,
    pub(crate) frames: u32,
    pub(crate) shot_requested: bool,
}

impl CleanerApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>, shot: Option<PathBuf>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG;
        visuals.window_fill = CARD;
        visuals.extreme_bg_color = CARD_SOFT;
        visuals.faint_bg_color = CARD_SOFT;
        visuals.window_stroke = Stroke::new(1.0_f32, LINE);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);
        visuals.widgets.inactive.bg_fill = CARD_SOFT;
        visuals.widgets.inactive.weak_bg_fill = CARD_SOFT;
        visuals.widgets.hovered.bg_fill = CARD_HOVER;
        visuals.widgets.hovered.weak_bg_fill = CARD_HOVER;
        visuals.widgets.active.bg_fill = ACCENT.gamma_multiply(0.6);
        visuals.selection.bg_fill = ACCENT.gamma_multiply(0.5);
        visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT2);
        visuals.hyperlink_color = ACCENT2;
        visuals.override_text_color = Some(TEXT);
        for w in [
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.open,
        ] {
            w.corner_radius = CornerRadius::ZERO;
        }
        cc.egui_ctx.set_visuals(visuals);

        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "archivo".to_owned(),
            Arc::new(FontData::from_static(include_bytes!("../Archivo.ttf"))),
        );
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "archivo".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        let mut sel = [false; Group::COUNT];
        for &g in Group::ALL.iter() {
            sel[gindex(g)] = matches!(
                g,
                Group::WindowsTemp
                    | Group::UserTemp
                    | Group::RecycleBin
                    | Group::ThumbnailCache
                    | Group::ErrorReports
                    | Group::WindowsLogs
                    | Group::BrowserCache
                    | Group::RecentDocs
                    | Group::Prefetch
                    | Group::ShaderCache
                    | Group::AppCaches
            );
        }

        // Rules are read once, at start: the built-in list first, then the user's
        // own files, which are allowed to replace a section of the same name.
        let wanted = rules::extra_files();
        let (rules, rule_files) = rules::load(&wanted);
        let rules = Arc::new(rules);
        let builtin_rules = rules::parse(rules::BUILTIN).len();
        let mut rule_sel: Vec<bool> = rules.iter().map(|r| r.present && r.default).collect();

        // Folders ticked and switches flipped last time win over the defaults above.
        let (ask_before, auto_analyze, lang, collapsed, custom, exclude, saved_rules) =
            match saved_settings(cc) {
                Some(s) => {
                    if s.sel.len() == sel.len() {
                        sel.copy_from_slice(&s.sel);
                    }
                    let custom = strings_to_paths(&s.custom);
                    let exclude = strings_to_paths(&s.exclude);
                    (
                        s.ask_before,
                        s.auto_analyze,
                        s.lang,
                        s.collapsed,
                        custom,
                        exclude,
                        s.rules,
                    )
                }
                None => (
                    true,
                    true,
                    Lang::default(),
                    false,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                ),
            };
        // A rule ticked last time stays ticked, but only if it still applies here.
        for (i, r) in rules.iter().enumerate() {
            if r.present && saved_rules.iter().any(|n| n == &r.name) {
                rule_sel[i] = true;
            }
        }
        Self {
            page: Page::Cleaner,
            tab: Tab::Windows,
            sel,
            search: String::new(),
            custom,
            exclude,
            new_custom: String::new(),
            new_exclude: String::new(),
            rules,
            rule_files,
            builtin_rules,
            rule_sel,
            report: None,
            scanning: false,
            scan_rx: None,
            scan_started: None,
            last_scan: None,
            status: String::new(),
            status_is_error: false,
            status_at: None,
            page_at: None,
            shown_page: None,
            cleaning: false,
            clean_rx: None,
            confirm_clean: false,
            pending_clean: None,
            ask_before,
            auto_analyze,
            analyzed_once: false,
            no_start_scan: false,
            open_confirm: false,
            lang,
            collapsed,
            reg_checks: [true, true, true],
            reg_issues: Vec::new(),
            reg_sel: Vec::new(),
            reg_running: false,
            reg_rx: None,
            reg_started: None,
            reg_scanned: false,
            reg_status: String::new(),
            last_reg_scan: None,
            sys: sysinfo::collect(),
            elevated: admin::is_elevated(),
            dry_run: false,
            shot,
            shot_started: Some(Instant::now()),
            frames: 0,
            shot_requested: false,
        }
    }

    pub(crate) fn chosen_groups(&self) -> Vec<Group> {
        Group::ALL
            .iter()
            .copied()
            .filter(|g| self.sel[gindex(*g)])
            .collect()
    }

    /// The application rules the user has ticked. A rule the list leaves out — one
    /// the scan found nothing in — is not offered either.
    pub(crate) fn chosen_rules(&self) -> Vec<usize> {
        self.rule_sel
            .iter()
            .enumerate()
            .filter(|(i, on)| {
                **on && self.rules.get(*i).is_some_and(|r| r.present)
                    && self.report.as_ref().is_none_or(|r| r.rule_totals(*i).1 > 0)
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Is this entry part of what the user has ticked?
    pub(crate) fn picked(&self, f: &clean::Found) -> bool {
        match f.rule {
            Some(i) => self.rule_sel.get(i).copied().unwrap_or(false),
            None => self.sel[gindex(f.group)],
        }
    }

    /// Every status line goes through here, so the fade-in always restarts.
    pub(crate) fn set_status(&mut self, text: impl Into<String>, error: bool) {
        self.status = text.into();
        self.status_is_error = error;
        self.status_at = Some(Instant::now());
    }

    pub(crate) fn start_scan(&mut self) {
        if self.scanning || self.cleaning {
            return;
        }
        let groups = self.chosen_groups();
        let rule_ids = self.chosen_rules();
        if groups.is_empty() && rule_ids.is_empty() {
            self.set_status(self.lang.t().tick_first, true);
            return;
        }
        let req = clean::Request {
            groups,
            rules: rule_ids,
            custom: self.custom.clone(),
            exclude: self.exclude.clone(),
        };
        let rules = Arc::clone(&self.rules);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rep = clean::scan(&req, &rules, 60_000);
            let _ = tx.send(rep);
        });
        self.scan_rx = Some(rx);
        self.scanning = true;
        self.scan_started = Some(Instant::now());
        self.set_status(self.lang.t().analyzing, false);
    }

    pub(crate) fn poll_scan(&mut self) {
        let Some(rx) = &self.scan_rx else { return };
        match rx.try_recv() {
            Ok(rep) => {
                let text = crate::i18n::fill(
                    self.lang.t().analysis_complete,
                    &[&rep.total_files().to_string(), &clean::human(rep.total())],
                );
                self.report = Some(rep);
                self.scanning = false;
                self.scan_rx = None;
                self.last_scan = Some(Instant::now());
                self.analyzed_once = true;
                self.set_status(text, false);
                // `--confirm`: ask once the scan has landed, so the list behind the
                // dialog holds real rows instead of "0 B".
                if self.open_confirm {
                    self.open_confirm = false;
                    self.pending_clean = Some((self.chosen_groups(), self.chosen_rules()));
                    self.confirm_clean = true;
                }
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.scanning = false;
                self.scan_rx = None;
                self.set_status(self.lang.t().analysis_stopped, true);
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    pub(crate) fn start_reg_scan(&mut self) {
        if self.reg_running {
            return;
        }
        let checks = self.reg_checks;
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let v = registry::scan(checks[0], checks[1], checks[2]);
            let _ = tx.send(v);
        });
        self.reg_rx = Some(rx);
        self.reg_running = true;
        self.reg_started = Some(Instant::now());
        self.reg_status = self.lang.t().looking_issues.into();
    }

    pub(crate) fn poll_reg(&mut self) {
        let Some(rx) = &self.reg_rx else { return };
        match rx.try_recv() {
            Ok(v) => {
                self.reg_status = if v.is_empty() {
                    self.lang.t().no_issues_found.to_string()
                } else {
                    self.lang
                        .t()
                        .issues_found
                        .replace("{}", &v.len().to_string())
                };
                self.reg_sel = vec![true; v.len()];
                self.reg_issues = v;
                self.reg_running = false;
                self.reg_rx = None;
                self.reg_scanned = true;
                self.last_reg_scan = Some(Instant::now());
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.reg_running = false;
                self.reg_rx = None;
                self.reg_status = self.lang.t().scan_stopped.into();
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    /// Removing files walks whole folder trees, which can take seconds; it runs on
    /// its own thread so the window keeps drawing the spinner and stays movable.
    ///
    /// `picked` is the set of ticks at the moment the button was pressed, carried
    /// across to the worker so a checkbox moved in the meantime cannot change what
    /// this run removes.
    pub(crate) fn do_clean(&mut self, groups: Vec<Group>, rules: Vec<usize>) {
        if self.cleaning || self.scanning {
            return;
        }
        let Some(rep) = self.report.clone() else {
            return;
        };
        let dry = self.dry_run;
        let rule_list = Arc::clone(&self.rules);
        let empty_bin = groups.contains(&Group::RecycleBin);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let want = |f: &clean::Found| match f.rule {
                Some(i) => rules.contains(&i),
                None => groups.contains(&f.group),
            };
            let res = if dry {
                clean::dry_run(&rep, &rule_list, &want, empty_bin)
            } else {
                clean::clean(&rep, &rule_list, &want, empty_bin)
            };
            let _ = tx.send((rep, groups, rules, res));
        });
        self.clean_rx = Some(rx);
        self.cleaning = true;
        self.set_status(self.lang.t().removing, false);
    }

    pub(crate) fn poll_clean(&mut self) {
        let Some(rx) = &self.clean_rx else { return };
        match rx.try_recv() {
            Ok((rep, groups, rules, res)) => {
                self.clean_rx = None;
                self.cleaning = false;
                self.finish_clean(rep, groups, rules, res);
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.clean_rx = None;
                self.cleaning = false;
                self.set_status(self.lang.t().clean_stopped, true);
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    /// Records the outcome once the worker thread is done.
    fn finish_clean(
        &mut self,
        rep: Report,
        groups: Vec<Group>,
        rules: Vec<usize>,
        res: clean::CleanResult,
    ) {
        // Anything we failed to remove stays listed so it can be tried again.
        let remaining: Vec<clean::Found> = rep
            .files
            .iter()
            .filter(|f| {
                let wanted = match f.rule {
                    Some(i) => rules.contains(&i),
                    None => groups.contains(&f.group),
                };
                res.dry_run || !wanted || f.path.exists()
            })
            .cloned()
            .collect();
        let mut report = Report {
            per_group: Vec::new(),
            files: remaining,
        };
        report.recount();
        self.report = Some(report);

        let recycled = if res.recycled > 0 {
            self.lang
                .t()
                .recycled_suffix
                .replace("{}", &res.recycled.to_string())
        } else {
            String::new()
        };
        let text = if res.dry_run {
            crate::i18n::fill(
                self.lang.t().dry_run_done,
                &[&res.done.to_string(), &clean::human(res.freed)],
            )
        } else if res.failed == 0 {
            crate::i18n::fill(
                self.lang.t().done_clean,
                &[&clean::human(res.freed), &res.done.to_string(), &recycled],
            )
        } else {
            crate::i18n::fill(
                self.lang.t().partial_clean,
                &[&clean::human(res.freed), &res.failed.to_string(), &recycled],
            )
        };
        self.last_scan = Some(Instant::now());
        self.set_status(text, res.failed > 0 && res.done == 0);
    }
}

/// The handful of choices worth remembering between runs.
#[derive(serde::Serialize, serde::Deserialize)]
struct Settings {
    #[serde(default = "yes")]
    ask_before: bool,
    #[serde(default = "yes")]
    auto_analyze: bool,
    #[serde(default)]
    sel: Vec<bool>,
    #[serde(default)]
    lang: Lang,
    #[serde(default)]
    collapsed: bool,
    /// Folders the user added, and folders to leave alone. Stored as text because
    /// serde has no path type that survives a round trip on every platform.
    #[serde(default)]
    custom: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
    /// The names of the application rules that were ticked. Names, not positions,
    /// because the rule list changes when a file is edited or updated.
    #[serde(default)]
    rules: Vec<String>,
}

fn strings_to_paths(v: &[String]) -> Vec<PathBuf> {
    v.iter().map(PathBuf::from).collect()
}

fn paths_to_strings(v: &[PathBuf]) -> Vec<String> {
    v.iter().map(|p| p.display().to_string()).collect()
}

fn yes() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ask_before: true,
            auto_analyze: true,
            sel: Vec::new(),
            lang: Lang::default(),
            collapsed: false,
            custom: Vec::new(),
            exclude: Vec::new(),
            rules: Vec::new(),
        }
    }
}

fn saved_settings(cc: &eframe::CreationContext<'_>) -> Option<Settings> {
    cc.storage
        .and_then(|s| eframe::get_value::<Settings>(s, eframe::APP_KEY))
}

impl eframe::App for CleanerApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let s = Settings {
            ask_before: self.ask_before,
            auto_analyze: self.auto_analyze,
            sel: self.sel.to_vec(),
            lang: self.lang,
            collapsed: self.collapsed,
            custom: paths_to_strings(&self.custom),
            exclude: paths_to_strings(&self.exclude),
            rules: self
                .rules
                .iter()
                .enumerate()
                .filter(|(i, r)| r.present && self.rule_sel.get(*i).copied().unwrap_or(false))
                .map(|(_, r)| r.name.clone())
                .collect(),
        };
        eframe::set_value(storage, eframe::APP_KEY, &s);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        self.frames = self.frames.saturating_add(1);
        self.poll_scan();
        self.poll_reg();
        self.poll_clean();

        // A page change restarts the fade, wherever the click came from.
        if self.shown_page != Some(self.page) {
            self.shown_page = Some(self.page);
            self.page_at = Some(Instant::now());
        }

        if self.auto_analyze && !self.no_start_scan && !self.analyzed_once && self.frames == 2 {
            // `auto_analyze` is the saved preference; it is only spent for this run.
            self.analyzed_once = true;
            self.start_scan();
        }

        // Folding is animated instead of snapped: one value drives the panel
        // width and every part of the sidebar that fades in or out.
        let openness = ctx.animate_bool_with_time_and_easing(
            egui::Id::new("sidebar_open"),
            !self.collapsed,
            0.18,
            egui::emath::easing::cubic_out,
        );
        egui::SidePanel::left("nav")
            .exact_width(egui::lerp(64.0..=210.0, openness))
            // Width comes from the fold button, not from dragging: without this
            // the edge still shows a resize cursor and then does nothing.
            .resizable(false)
            .frame(
                egui::Frame::default()
                    .fill(NAV)
                    .inner_margin(egui::Margin::symmetric(0, 16)),
            )
            .show(ctx, |ui| self.sidebar(ui, openness));

        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::default()
                    .fill(NAV)
                    .inner_margin(egui::Margin::symmetric(18, 8)),
            )
            .show(ctx, |ui| self.status_bar(ui));

        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(24, 18)),
            )
            .show(ctx, |ui| {
                page_background(ui.painter(), ui.max_rect().expand(60.0));
                let has_actions = self.dry_run || !self.elevated;
                if ui.available_width() >= 560.0 {
                    // Actions claim the right edge first, so the heading gets exactly
                    // the width that is left and can never run underneath them.
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if has_actions {
                                self.header_actions(ui);
                            }
                            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                                self.page_heading(ui);
                            });
                        });
                    });
                } else {
                    // Too narrow for one row: heading first, actions beneath it.
                    self.page_heading(ui);
                    if has_actions {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                self.header_actions(ui);
                            });
                        });
                    }
                }
                ui.add_space(12.0);
                let (rule, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
                ui.painter().rect_filled(rule, CornerRadius::ZERO, LINE);
                ui.add_space(14.0);
                // The Cleaner page brings its own scrolling list and a footer pinned
                // to the bottom, so it fills this panel directly instead of the shared
                // scroll area below. The fade is the same, minus that wrapper.
                if self.page == Page::Cleaner {
                    ui.set_width(ui.available_width());
                    ui.set_opacity(crate::ui::fade_in(self.page_at));
                    self.cleaner_page(ui);
                    return;
                }
                // One scroll area for every other page, so a small window can always
                // reach the whole page instead of clipping whatever does not fit. Each
                // page keeps its own offset: a jump to a short page cannot land scrolled.
                egui::ScrollArea::vertical()
                    .id_salt(("page_scroll", self.page))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        // Cross-fade only: no slide, so switching pages cannot read
                        // as a bounce. The content stays put and just comes up to full.
                        ui.set_opacity(crate::ui::fade_in(self.page_at));
                        match self.page {
                            Page::Cleaner => {}
                            Page::Registry => self.registry_page(ui),
                            Page::Tools => self.tools_page(ui),
                            Page::Options => self.options_page(ui),
                            Page::Info => self.info_page(ui),
                        }
                    });
            });

        if self.confirm_clean {
            self.confirm_dialog(ctx);
        }

        if self.shot.is_some() {
            self.screenshot_step(ctx);
        }
    }
}

impl CleanerApp {
    /// The page title and its one-line explanation.
    fn page_heading(&self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(self.page.title(self.lang))
                    .size(crate::ui::type_size::PAGE_TITLE)
                    .strong()
                    .color(TEXT),
            );
            ui.add_space(2.0);
            ui.add(
                egui::Label::new(
                    RichText::new(self.page.subtitle(self.lang))
                        .size(crate::ui::type_size::BODY)
                        .color(MUTED),
                )
                .wrap(),
            );
        });
    }

    /// Dry-run badge and the elevation offer (version badge and toggle button removed).
    fn header_actions(&mut self, ui: &mut egui::Ui) {
        if self.dry_run {
            let (tag, _) = ui.allocate_exact_size(Vec2::new(96.0, 26.0), Sense::hover());
            ui.painter().rect_filled(tag, CornerRadius::ZERO, CARD_SOFT);
            ui.painter().text(
                tag.center(),
                egui::Align2::CENTER_CENTER,
                self.lang.t().dry_run,
                FontId::proportional(11.0),
                ACCENT2,
            );
        }
        if !self.elevated {
            // Some folders only open to an administrator; offer the upgrade
            // instead of silently reporting failures.
            let btn = egui::Button::new(
                RichText::new(self.lang.t().run_as_admin)
                    .size(11.0)
                    .color(MUTED),
            )
            .fill(CARD_SOFT)
            .stroke(Stroke::new(1.0_f32, LINE))
            .corner_radius(CornerRadius::ZERO);
            if ui.add(btn).clicked() {
                admin::restart_elevated();
            }
        }
    }

    pub(crate) fn screenshot_step(&mut self, ctx: &egui::Context) {
        let target = self.shot.clone().unwrap_or_default();
        let waited_long_enough = self
            .shot_started
            .map(|s| s.elapsed().as_secs_f32() > 30.0)
            .unwrap_or(false);
        // Leave the picture until the analysis has settled, so it shows real numbers,
        // but never wait forever on a slow computer.
        let ready =
            self.frames >= 120 && (!self.scanning || self.report.is_some() || waited_long_enough);
        if !self.shot_requested && ready {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.shot_requested = true;
        }
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = shot {
            if let Some(dir) = target.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let [w, h] = img.size;
            let mut bytes = Vec::with_capacity(w * h * 4);
            for px in img.pixels.iter() {
                let [r, g, b, a] = px.to_array();
                bytes.extend_from_slice(&[r, g, b, a]);
            }
            let saved = image::RgbaImage::from_raw(w as u32, h as u32, bytes)
                .map(|buf| buf.save(&target).is_ok())
                .unwrap_or(false);
            if !saved {
                eprintln!("could not save {}", target.display());
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            self.shot = None;
        } else if self.frames > 1200 {
            eprintln!("no picture arrived; closing");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            self.shot = None;
        }
    }
}
