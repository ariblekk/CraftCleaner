//! Everything that draws: the palette, the sidebar, and the five pages.
//!
//! Kept apart from `app.rs` so that what the program does and how it looks can be
//! read, and changed, without walking past each other.

use crate::app::CleanerApp;
use crate::clean::{self, Group, Tab};
use crate::i18n::{self, Lang};
use crate::{open_path, registry, sysinfo};
use eframe::egui;
use egui::{Align, Color32, CornerRadius, FontId, Layout, Rect, RichText, Sense, Stroke, Vec2};
use egui_animation as anim;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Easing curves and the animation helpers, re-exported by `egui_animation`.
use anim::easing as ease;
// --- palette -----------------------------------------------------------------
// Linear-style: flat neutral dark greys, one quiet indigo accent, no glow, no rounded corners.
pub(crate) const BG: Color32 = Color32::from_rgb(0x0F, 0x10, 0x11); // page
pub(crate) const CARD: Color32 = Color32::from_rgb(0x14, 0x15, 0x17);
pub(crate) const CARD_HOVER: Color32 = Color32::from_rgb(0x1A, 0x1B, 0x1E);
pub(crate) const CARD_SOFT: Color32 = Color32::from_rgb(0x1C, 0x1D, 0x21);
pub(crate) const NAV: Color32 = Color32::from_rgb(0x0B, 0x0C, 0x0D);
pub(crate) const NAV_HOVER: Color32 = Color32::from_rgb(0x17, 0x18, 0x1B);
pub(crate) const NAV_SEL: Color32 = Color32::from_rgb(0x1B, 0x1C, 0x20);
pub(crate) const NAV_TEXT: Color32 = Color32::from_rgb(0x8A, 0x8F, 0x98);
pub(crate) const ACCENT: Color32 = Color32::from_rgb(0x5E, 0x6A, 0xD2); // muted indigo
pub(crate) const ACCENT_HOVER: Color32 = Color32::from_rgb(0x6E, 0x7A, 0xE0);
pub(crate) const ACCENT2: Color32 = Color32::from_rgb(0x8A, 0x8F, 0x98); // secondary text-grey, no second hue
pub(crate) const TEXT: Color32 = Color32::from_rgb(0xE7, 0xE8, 0xEA);
pub(crate) const MUTED: Color32 = Color32::from_rgb(0x7C, 0x81, 0x89);
pub(crate) const LINE: Color32 = Color32::from_rgb(0x26, 0x28, 0x2C);
pub(crate) const WARN: Color32 = Color32::from_rgb(0xD9, 0x6A, 0x6A);

/// Shared typography scale, in egui points.
pub(crate) mod type_size {
    pub const PAGE_TITLE: f32 = 17.0;
    pub const BODY: f32 = 12.0;
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Page {
    Cleaner,
    Registry,
    Tools,
    Options,
    Info,
}

impl Page {
    pub(crate) fn title(self, lang: Lang) -> &'static str {
        let t = lang.t();
        match self {
            Page::Cleaner => t.page_cleaner,
            Page::Registry => t.page_registry,
            Page::Tools => t.page_tools,
            Page::Options => t.page_options,
            Page::Info => t.page_info,
        }
    }
    pub(crate) fn subtitle(self, lang: Lang) -> &'static str {
        let t = lang.t();
        match self {
            Page::Cleaner => t.sub_cleaner,
            Page::Registry => t.sub_registry,
            Page::Tools => t.sub_tools,
            Page::Options => t.sub_options,
            Page::Info => t.sub_info,
        }
    }
}

pub(crate) fn gindex(g: Group) -> usize {
    Group::ALL.iter().position(|x| *x == g).unwrap_or(0)
}

/// One heading and the categories under it, as shown on each tab.
pub(crate) fn layout_for(tab: Tab, lang: Lang) -> Vec<(&'static str, Vec<Group>)> {
    let t = lang.t();
    match tab {
        Tab::Windows => vec![
            (
                t.head_windows_explorer,
                vec![Group::RecentDocs, Group::ThumbnailCache],
            ),
            (
                t.head_system,
                vec![
                    Group::WindowsTemp,
                    Group::UserTemp,
                    Group::RecycleBin,
                    Group::WindowsLogs,
                    Group::ErrorReports,
                    Group::MemoryDumps,
                ],
            ),
            (
                t.head_advanced,
                vec![
                    Group::Prefetch,
                    Group::FontCache,
                    Group::ShaderCache,
                    Group::Custom,
                ],
            ),
        ],
        Tab::Applications => vec![
            (
                t.head_internet,
                vec![Group::BrowserCache, Group::BrowserCookies],
            ),
            (t.head_apps, vec![Group::AppCaches]),
            (t.head_downloads, vec![Group::Downloads]),
            // The application rules live here too, drawn from the loaded list
            // rather than from a fixed category: see `cleaner_page`.
            (t.head_app_rules, vec![Group::AppRule]),
        ],
        Tab::Advanced => vec![(
            t.head_windows,
            vec![
                Group::WindowsUpdateCache,
                Group::DeliveryOptimization,
                Group::WindowsOld,
            ],
        )],
    }
}

/// The groups on one tab, in the order the rows appear. The application-rule
/// group is left out: it is drawn from the loaded rule list, not ticked by group.
pub(crate) fn groups_on(tab: Tab, lang: Lang) -> Vec<Group> {
    layout_for(tab, lang)
        .into_iter()
        .flat_map(|(_, gs)| gs)
        .filter(|g| *g != Group::AppRule)
        .collect()
}

/// Does a category match what was typed in the search box? Empty always matches.
pub(crate) fn group_matches(lang: Lang, g: Group, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    i18n::group_label(lang, g).to_lowercase().contains(&q)
        || i18n::group_note(lang, g).to_lowercase().contains(&q)
}

// --- drawing helpers ---------------------------------------------------------

/// One row of text, cut with an ellipsis rather than wrapped, for fixed-height rows.
fn one_line(text: &str, font: FontId, colour: Color32, max_w: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::single_section(
        text.to_owned(),
        egui::TextFormat::simple(font, colour),
    );
    job.wrap = egui::text::TextWrapping {
        max_width: max_w,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    job
}

/// Eased 0..1 fade for something that has just appeared at `at`.
pub(crate) fn fade_in(at: Option<Instant>) -> f32 {
    let t = at
        .map(|s| (s.elapsed().as_secs_f32() / 0.20).min(1.0))
        .unwrap_or(1.0);
    ease::quad_out(t)
}

/// A ring of ticks whose lit part travels round, drawn with the shared easing.
fn spinner(ui: &mut egui::Ui, size: f32) {
    let t = anim::animate_repeating(ui, ease::linear, Duration::from_millis(1100), 0.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let center = rect.center();
    let radius = size * 0.5 - 1.0;
    let p = ui.painter();
    for i in 0..8 {
        // Every tick is a quarter turn behind the one before it.
        let phase = (t - i as f32 / 8.0).rem_euclid(1.0);
        let a = std::f32::consts::TAU * (i as f32 / 8.0);
        let dir = Vec2::new(a.cos(), a.sin());
        p.line_segment(
            [center + dir * (radius - 2.5), center + dir * radius],
            Stroke::new(1.8_f32, MUTED.gamma_multiply(0.25 + 0.75 * (1.0 - phase))),
        );
    }
}

/// Blend two colours; `t = 0` gives `a`, `t = 1` gives `b`.
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        ch(a.r(), b.r()),
        ch(a.g(), b.g()),
        ch(a.b(), b.b()),
        ch(a.a(), b.a()),
    )
}

/// Eased hover in 0..1, remembered per widget so the colour does not snap.
fn hover(ui: &egui::Ui, resp: &egui::Response) -> f32 {
    anim::animate_bool_eased(
        ui.ctx(),
        resp.id.with("hover"),
        resp.hovered(),
        ease::quad_out,
        0.12,
    )
}

/// Flat page background: Linear's look is a single solid dark surface, no gradient or glow.
pub(crate) fn page_background(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, CornerRadius::ZERO, BG);
}

pub(crate) fn card(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::default()
        .fill(CARD)
        .corner_radius(CornerRadius::ZERO)
        .stroke(Stroke::new(1.0_f32, LINE))
        .inner_margin(egui::Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            content(ui);
        });
}

/// How wide a text button must be to hold its label, so a longer word in one
/// language cannot spill past the box. Never narrower than `min`.
fn button_width(ui: &egui::Ui, text: &str) -> f32 {
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_string(), FontId::proportional(13.5), Color32::WHITE);
    (galley.size().x + 28.0).max(96.0)
}

pub(crate) fn primary_button(ui: &mut egui::Ui, text: &str, size: Vec2) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let h = hover(ui, &resp);
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, mix(ACCENT, ACCENT_HOVER, h));
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        FontId::proportional(13.5),
        Color32::from_rgb(0xFF, 0xFF, 0xFF),
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(crate) fn ghost_button(ui: &mut egui::Ui, text: &str, size: Vec2) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let h = hover(ui, &resp);
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, mix(CARD, CARD_HOVER, h));
    ui.painter().rect_stroke(
        rect,
        CornerRadius::ZERO,
        Stroke::new(1.0_f32, mix(LINE, MUTED, h)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        FontId::proportional(13.5),
        mix(MUTED, TEXT, h),
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// One tickable row: checkbox, a title, a note, and an optional "size · count"
/// badge on the right. Used for both built-in categories and application rules,
/// so the two look identical and only the data differs.
///
/// Returns true when the row was clicked and may be toggled.
fn row(
    ui: &mut egui::Ui,
    on: bool,
    available: bool,
    title: &str,
    note: &str,
    badge: Option<String>,
) -> bool {
    let row_h = 46.0;
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::click());
    let h = hover(ui, &resp);
    if h > 0.01 {
        ui.painter().rect_filled(
            rect,
            CornerRadius::ZERO,
            mix(Color32::TRANSPARENT, CARD_HOVER, h),
        );
    }
    let box_rect = Rect::from_center_size(
        egui::pos2(rect.left() + 16.0, rect.center().y),
        Vec2::splat(17.0),
    );
    let fill = if on { ACCENT } else { CARD_SOFT };
    ui.painter().rect_filled(box_rect, CornerRadius::ZERO, fill);
    ui.painter().rect_stroke(
        box_rect,
        CornerRadius::ZERO,
        Stroke::new(1.0_f32, if on { ACCENT } else { LINE }),
        egui::StrokeKind::Inside,
    );
    if on {
        let c = box_rect.center();
        ui.painter().line_segment(
            [egui::pos2(c.x - 4.5, c.y), egui::pos2(c.x - 1.0, c.y + 3.5)],
            Stroke::new(2.0_f32, Color32::from_rgb(0x0E, 0x0F, 0x14)),
        );
        ui.painter().line_segment(
            [
                egui::pos2(c.x - 1.0, c.y + 3.5),
                egui::pos2(c.x + 5.0, c.y - 4.0),
            ],
            Stroke::new(2.0_f32, Color32::from_rgb(0x0E, 0x0F, 0x14)),
        );
    }

    // The count badge claims its width first, and both text lines are cut off
    // before it, so a long label or note can never spill out of the row.
    let badge_w = if rect.width() < 520.0 { 92.0 } else { 112.0 };
    let text_w = if badge.is_none() {
        rect.width() - 32.0 - 12.0
    } else {
        rect.width() - 32.0 - badge_w - 24.0
    }
    .max(40.0);
    let text_colour = if available { TEXT } else { MUTED };
    let title = ui.painter().layout_job(one_line(
        title,
        FontId::proportional(13.0),
        text_colour,
        text_w,
    ));
    ui.painter().galley(
        egui::pos2(
            rect.left() + 32.0,
            rect.center().y - 8.0 - title.size().y / 2.0,
        ),
        title,
        text_colour,
    );
    // One line only, cut with an ellipsis: a clamped note beats a row that grows.
    let note = ui
        .painter()
        .layout_job(one_line(note, FontId::proportional(10.0), MUTED, text_w));
    ui.painter().galley(
        egui::pos2(
            rect.left() + 32.0,
            rect.center().y + 9.0 - note.size().y / 2.0,
        ),
        note,
        MUTED,
    );
    if let Some(text) = badge {
        // The badge is measured from its own text, so nothing leaves the box —
        // a long "size · count" only gives way to the ellipsis at the far edge.
        // `layout_no_wrap`: the galley must not inherit a tighter wrap width
        // from the scroll area, which would break the line before its edge.
        let galley = ui
            .painter()
            .layout_no_wrap(text, FontId::proportional(11.0), ACCENT2);
        let w = (galley.size().x + 24.0).max(96.0).min(badge_w);
        let tag_rect = Rect::from_min_size(
            egui::pos2(rect.right() - w - 12.0, rect.center().y - 12.0),
            Vec2::new(w, 24.0),
        );
        ui.painter()
            .rect_filled(tag_rect, CornerRadius::ZERO, CARD_SOFT);
        ui.painter().galley(
            egui::pos2(
                tag_rect.center().x - galley.size().x / 2.0,
                tag_rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            ACCENT2,
        );
    }
    resp.clicked()
}

/// One line of the rule-files card: a dot, a label, and the section count on the
/// right. Used for the built-in list and for each file that was read.
fn rule_file_line(ui: &mut egui::Ui, label: &str, sections: String, colour: egui::Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("\u{2022}").size(12.0).color(colour));
        ui.add_space(2.0);
        ui.label(RichText::new(label).size(11.0).color(TEXT));
        ui.add_space(6.0);
        ui.label(RichText::new(sections).size(10.0).color(MUTED));
    });
}

/// A quiet tab label with a 2px underline when selected, the way Linear does tabs.
pub(crate) fn pill(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        FontId::proportional(13.0),
        if selected { TEXT } else { MUTED },
    );
    let size = Vec2::new(galley.size().x + 24.0, 30.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if selected {
        ui.painter().rect_filled(
            Rect::from_min_max(
                egui::pos2(rect.left(), rect.bottom() - 2.0),
                rect.right_bottom(),
            ),
            CornerRadius::ZERO,
            ACCENT,
        );
    } else {
        let h = hover(ui, &resp);
        ui.painter().rect_filled(
            rect,
            CornerRadius::ZERO,
            mix(Color32::TRANSPARENT, CARD_HOVER, h),
        );
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        FontId::proportional(13.0),
        if selected { TEXT } else { MUTED },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(crate) fn nav_glyph(which: Page) -> &'static str {
    use egui_phosphor::regular as icon;
    match which {
        Page::Cleaner => icon::BROOM,
        Page::Registry => icon::LIST_BULLETS,
        Page::Tools => icon::WRENCH,
        Page::Options => icon::SLIDERS_HORIZONTAL,
        Page::Info => icon::INFO,
    }
}

/// Sidebar fold button inside the sidebar header.
fn sidebar_toggle(ui: &mut egui::Ui, glyph: &str, tip: &str, alpha: f32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(28.0, 26.0), Sense::click());
    let h = hover(ui, &resp);
    if h > 0.01 {
        ui.painter().rect_filled(
            rect,
            CornerRadius::ZERO,
            mix(Color32::TRANSPARENT, NAV_HOVER, h).gamma_multiply(alpha),
        );
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(15.0),
        mix(NAV_TEXT, TEXT, h).gamma_multiply(alpha),
    );
    resp.on_hover_text(tip)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

impl CleanerApp {
    /// `t` is how far the sidebar is open: 0 folded to icons, 1 fully open.
    /// Every part that appears or disappears is drawn at `t` opacity and takes
    /// `t` of its height, so folding and unfolding have no jumps.
    pub(crate) fn sidebar(&mut self, ui: &mut egui::Ui, t: f32) {
        let folded = 1.0 - t;
        let tr = self.lang.t();
        ui.add_space(6.0);

        // No brand block: the window title says what this is, so the top of the
        // sidebar is only the fold switch. It sits right when open and centred when
        // folded, which is where the caret to unfold it lives.
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), 30.0),
            if folded > 0.5 {
                Layout::top_down(Align::Center)
            } else {
                Layout::right_to_left(Align::Center)
            },
            |ui| {
                if folded > 0.5 {
                    // Faded with `folded`, not `t`: at `t = 0` the switch would vanish.
                    if sidebar_toggle(ui, egui_phosphor::regular::CARET_RIGHT, tr.expand, folded) {
                        self.collapsed = false;
                    }
                } else {
                    ui.add_space(12.0);
                    if sidebar_toggle(ui, egui_phosphor::regular::SIDEBAR_SIMPLE, tr.collapse, t) {
                        self.collapsed = true;
                    }
                }
            },
        );

        ui.add_space(6.0 + 8.0 * t);

        // The section label keeps its own height, scaled by `t`, so the items
        // below slide up instead of jumping when it disappears.
        let band = 20.0 * t;
        if band > 0.5 {
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), band),
                Layout::top_down(Align::LEFT),
                |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(18.0);
                        ui.label(
                            RichText::new(tr.workspace)
                                .size(9.5)
                                .strong()
                                .color(NAV_TEXT.gamma_multiply(t)),
                        );
                    });
                },
            );
        }
        ui.add_space(6.0 * t);

        for page in [
            Page::Cleaner,
            Page::Registry,
            Page::Tools,
            Page::Options,
            Page::Info,
        ] {
            let selected = self.page == page;
            let height = 32.0 + 6.0 * (1.0 - t);
            // full-width flat row, exactly like a Linear sidebar item
            let (rect, resp) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
            let bg = if selected {
                NAV_SEL
            } else {
                mix(Color32::TRANSPARENT, NAV_HOVER, hover(ui, &resp))
            };
            ui.painter().rect_filled(rect, CornerRadius::ZERO, bg);
            if selected {
                ui.painter().rect_filled(
                    Rect::from_min_size(rect.min, Vec2::new(2.0, rect.height())),
                    CornerRadius::ZERO,
                    ACCENT,
                );
            }
            let fg = if selected { TEXT } else { NAV_TEXT };
            // The glyph slides between the closed column's centre and its open spot.
            let glyph_x = egui::lerp(24.0..=(rect.width() / 2.0), 1.0 - t);
            ui.painter().text(
                egui::pos2(rect.left() + glyph_x, rect.center().y),
                egui::Align2::CENTER_CENTER,
                nav_glyph(page),
                FontId::proportional(15.0 + (1.0 - t)),
                fg,
            );
            if t > 0.05 {
                ui.painter().text(
                    egui::pos2(rect.left() + 42.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    page.title(self.lang),
                    FontId::proportional(13.0),
                    fg.gamma_multiply(t),
                );
            }
            // Closed, the glyph is all there is, so it carries the tooltip.
            let resp = if t < 0.5 {
                resp.on_hover_text(page.title(self.lang))
            } else {
                resp
            };
            if resp.clicked() {
                self.page = page;
                if page == Page::Info {
                    self.sys = sysinfo::collect();
                }
            }
        }

        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            // The version, at the very foot of the rail. Folded, the column is too
            // narrow for even that, so it steps out of the way rather than being cut.
            if t > 0.5 {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
                    ui.label(
                        RichText::new("v1.0")
                            .size(10.0)
                            .color(MUTED.gamma_multiply(t)),
                    );
                });
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                // The dot moves from the closed column's centre to the open row.
                let pad = (ui.available_width() / 2.0 - 22.0) * folded;
                ui.add_space(18.0 + pad);
                let (dot, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                ui.painter().rect_filled(dot, CornerRadius::ZERO, ACCENT);
                if t > 0.05 {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(&self.sys.user)
                            .size(11.0)
                            .color(TEXT.gamma_multiply(t)),
                    );
                    ui.label(
                        RichText::new(&self.sys.host)
                            .size(10.0)
                            .color(MUTED.gamma_multiply(t)),
                    );
                }
            });
        });
    }

    pub(crate) fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // The disk figure claims the right edge first and the message gets the
            // rest, so on a small window the two can never end up on top of one
            // another, and the free-space line simply steps out of the way.
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.available_width() > 460.0 {
                    let t = self.lang.t();
                    ui.label(
                        RichText::new(i18n::fill(
                            t.free_of,
                            &[
                                &crate::clean::human(self.sys.disk_free),
                                &crate::clean::human(self.sys.disk_total),
                            ],
                        ))
                        .size(11.0)
                        .color(MUTED),
                    );
                }
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    self.status_line(ui);
                });
            });
        });
    }

    fn status_line(&mut self, ui: &mut egui::Ui) {
        let t = self.lang.t();
        {
            if self.scanning || self.reg_running || self.cleaning {
                spinner(ui, 14.0);
                ui.label(
                    RichText::new(if self.scanning {
                        t.analyzing
                    } else if self.cleaning {
                        t.removing
                    } else {
                        t.looking_issues
                    })
                    .size(12.0)
                    .color(MUTED),
                );
                // No real progress is known, so the bar sweeps instead of lying:
                // a lit segment travels right and loops with the eased curve.
                let phase = anim::animate_repeating(
                    ui,
                    ease::quad_in_out,
                    Duration::from_millis(1400),
                    0.0,
                );
                let (bar, _) = ui.allocate_exact_size(Vec2::new(140.0, 4.0), Sense::hover());
                ui.painter().rect_filled(bar, CornerRadius::ZERO, CARD_SOFT);
                let w = bar.width() * 0.3;
                let x = bar.left() + (bar.width() - w) * phase;
                ui.painter().rect_filled(
                    Rect::from_min_size(egui::pos2(x, bar.top()), Vec2::new(w, bar.height())),
                    CornerRadius::ZERO,
                    ACCENT,
                );
                ui.ctx().request_repaint();
            } else if self.status.is_empty() {
                if let Some(last) = self.last_scan {
                    let secs = last.elapsed().as_secs();
                    let text = if secs < 5 {
                        t.analysis_just_finished.to_string()
                    } else if secs < 90 {
                        t.last_analysis_secs.replace("{}", &secs.to_string())
                    } else {
                        t.last_analysis_mins.replace("{}", &(secs / 60).to_string())
                    };
                    ui.label(RichText::new(text).size(12.0).color(MUTED));
                } else {
                    ui.label(RichText::new(t.ready).size(12.0).color(MUTED));
                }
            } else {
                let colour = if self.status_is_error { WARN } else { TEXT };
                // Eased in from nothing so a new line announces itself quietly.
                let alpha = fade_in(self.status_at);
                ui.add(
                    egui::Label::new(
                        RichText::new(&self.status)
                            .size(12.0)
                            .color(colour.gamma_multiply(alpha)),
                    )
                    .wrap(),
                );
            }
        }
    }

    /// The Cleaner page has two faces. Before a scan there is one thing to do, so
    /// the page is an empty state with a single button. After a scan the same page
    /// becomes a list: the tabs and the search stay put, only the rows scroll, and
    /// what has been ticked plus the two buttons sit in a bar stuck to the bottom.
    pub(crate) fn cleaner_page(&mut self, ui: &mut egui::Ui) {
        // No report and nothing running: the empty state, and nothing else.
        if self.report.is_none() {
            if self.scanning {
                self.cleaner_scanning(ui);
            } else {
                self.cleaner_empty(ui);
            }
            return;
        }
        // Painted first so it keeps the bottom edge while the rows scroll under it.
        self.cleaner_footer(ui);
        self.cleaner_controls(ui);
        ui.add_space(8.0);
        // The one scrolling part of the page; the panels around it never move.
        egui::ScrollArea::vertical()
            .id_salt("cleaner_rows")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                self.cleaner_list(ui);
            });
    }

    /// The first screen: what the scan does, and the button that does it.
    fn cleaner_empty(&mut self, ui: &mut egui::Ui) {
        let t = self.lang.t();
        ui.add_space((ui.available_height() - 200.0).max(0.0) * 0.35);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t.empty_title).size(18.0).strong().color(TEXT));
            ui.add_space(6.0);
            ui.add(
                egui::Label::new(RichText::new(t.empty_body).size(12.0).color(MUTED))
                    .wrap()
                    .halign(Align::Center),
            );
            ui.add_space(18.0);
            let w = (button_width(ui, t.scan_now) + 48.0).min(ui.available_width());
            if primary_button(ui, t.scan_now, Vec2::new(w, 38.0)).clicked() {
                self.start_scan();
            }
        });
    }

    /// The same screen while the scan runs, so the click has an answer at once.
    fn cleaner_scanning(&mut self, ui: &mut egui::Ui) {
        let t = self.lang.t();
        ui.add_space((ui.available_height() - 200.0).max(0.0) * 0.35);
        ui.vertical_centered(|ui| {
            spinner(ui, 26.0);
            ui.add_space(10.0);
            ui.label(
                RichText::new(t.scanning_title)
                    .size(15.0)
                    .strong()
                    .color(TEXT),
            );
            ui.add_space(6.0);
            ui.add(
                egui::Label::new(RichText::new(t.scanning_body).size(12.0).color(MUTED))
                    .wrap()
                    .halign(Align::Center),
            );
        });
    }

    /// Tabs and search, fixed above the rows once there is something to show.
    fn cleaner_controls(&mut self, ui: &mut egui::Ui) {
        let t = self.lang.t();
        ui.horizontal_wrapped(|ui| {
            for tab in Tab::ALL {
                if pill(ui, i18n::tab_label(self.lang, tab), self.tab == tab).clicked() {
                    self.tab = tab;
                }
                ui.add_space(8.0);
            }
        });
        ui.add_space(10.0);
        // Search and select-all/none: one row when it fits, otherwise the search
        // takes a row of its own and the two buttons share the next. The buttons are
        // measured from their labels, so a longer word in one language still fits.
        let all_w = button_width(ui, t.select_all);
        let none_w = button_width(ui, t.select_none);
        let buttons_w = all_w + 6.0 + none_w;
        let buttons = |app: &mut CleanerApp, ui: &mut egui::Ui| {
            if ghost_button(ui, t.select_all, Vec2::new(all_w, 28.0)).clicked() {
                app.select_tab(app.tab, true);
            }
            ui.add_space(6.0);
            if ghost_button(ui, t.select_none, Vec2::new(none_w, 28.0)).clicked() {
                app.select_tab(app.tab, false);
            }
        };
        if ui.available_width() >= buttons_w + 220.0 {
            ui.horizontal(|ui| {
                self.search_field(ui, ui.available_width() - buttons_w - 8.0);
                ui.add_space(8.0);
                buttons(self, ui);
            });
        } else {
            self.search_field(ui, ui.available_width());
            ui.add_space(8.0);
            ui.horizontal(|ui| buttons(self, ui));
        }
    }

    fn search_field(&mut self, ui: &mut egui::Ui, width: f32) {
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text(self.lang.t().search_hint)
                .desired_width(width.max(120.0)),
        );
    }

    /// Tick or untick every row on the tab in view. This is the one way back to a
    /// row that was unticked: unticking never hides anything.
    ///
    /// Every row on the tab is always on screen, found files or none, so the whole
    /// tab gets ticked. A category that was unticked before the scan was never
    /// looked at, so its count is zero; skipping on the count would leave it
    /// untickable forever, which is what made "select all" look dead.
    pub(crate) fn select_tab(&mut self, tab: Tab, on: bool) {
        for g in groups_on(tab, self.lang) {
            // A row the user cannot reach stays untickable, same as a click on it.
            let available = if g == Group::Custom {
                !self.custom.is_empty()
            } else {
                crate::clean::available(g)
            };
            if on && !available {
                continue;
            }
            self.sel[gindex(g)] = on;
        }
        if tab == Tab::Applications {
            for (i, slot) in self.rule_sel.iter_mut().enumerate() {
                // Only rules that apply on this machine are drawn, so only those
                // can be ticked; the ones that found nothing are still rows.
                if on && !self.rules.get(i).is_some_and(|r| r.present) {
                    continue;
                }
                *slot = on;
            }
        }
    }

    /// The sticky bar: how much is ticked, then run and rescan. Stacks into three
    /// rows when the window is too narrow for one.
    fn cleaner_footer(&mut self, ui: &mut egui::Ui) {
        let selected = self
            .report
            .as_ref()
            .map(|r| r.selected_total(&|f| self.picked(f)))
            .unwrap_or(0);
        let (found_files, found_bytes) = self
            .report
            .as_ref()
            .map(|r| (r.total_files(), r.total()))
            .unwrap_or((0, 0));
        let busy = self.scanning || self.cleaning;
        egui::TopBottomPanel::bottom("cleaner_footer")
            .resizable(false)
            .frame(
                egui::Frame::default()
                    .fill(CARD)
                    .inner_margin(egui::Margin::symmetric(18, 10)),
            )
            .show_inside(ui, |ui| {
                let line = ui.available_width() >= 560.0;
                if line {
                    ui.horizontal(|ui| {
                        self.footer_total(ui, selected, found_files, found_bytes);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            self.footer_buttons(ui, busy);
                        });
                    });
                } else {
                    self.footer_total(ui, selected, found_files, found_bytes);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        self.footer_buttons(ui, busy);
                    });
                }
            });
    }

    fn footer_total(&self, ui: &mut egui::Ui, selected: u64, found_files: usize, found_bytes: u64) {
        let t = self.lang.t();
        ui.vertical(|ui| {
            let head = if selected == 0 {
                t.nothing_selected.to_string()
            } else {
                i18n::fill(t.selected_total, &[&clean::human(selected)])
            };
            ui.label(RichText::new(head).size(14.0).strong().color(TEXT));
            ui.add_space(2.0);
            ui.label(
                RichText::new(i18n::fill(
                    t.found_total,
                    &[&found_files.to_string(), &clean::human(found_bytes)],
                ))
                .size(10.5)
                .color(MUTED),
            );
        });
    }

    /// Run on the right, rescan beside it; a busy app shows what it is doing
    /// instead, so a second click cannot start the same work twice.
    fn footer_buttons(&mut self, ui: &mut egui::Ui, busy: bool) {
        let t = self.lang.t();
        if busy {
            let label = if self.cleaning {
                t.removing
            } else {
                t.analyzing
            };
            ui.horizontal(|ui| {
                spinner(ui, 16.0);
                ui.add_space(6.0);
                ui.label(RichText::new(label).size(11.5).color(MUTED));
            });
            return;
        }
        if primary_button(
            ui,
            t.run_cleaner,
            Vec2::new(button_width(ui, t.run_cleaner), 32.0),
        )
        .clicked()
        {
            self.run_cleaner();
        }
        ui.add_space(6.0);
        if ghost_button(
            ui,
            t.scan_again,
            Vec2::new(button_width(ui, t.scan_again), 32.0),
        )
        .clicked()
        {
            self.start_scan();
        }
    }

    /// The rows themselves: application rules are their own block, everything
    /// else is a heading and a card of category rows.
    fn cleaner_list(&mut self, ui: &mut egui::Ui) {
        let t = self.lang.t();
        let mut any_shown = false;
        for (heading, groups) in layout_for(self.tab, self.lang) {
            // The application-rule group is not one row: it is every loaded rule
            // that applies on this machine, each with its own tick.
            if groups.contains(&Group::AppRule) {
                let shown: Vec<usize> = self
                    .rules
                    .iter()
                    .enumerate()
                    // Every rule that applies on this machine stays listed, ticked or
                    // not, with or without files found: a row that vanished on a click
                    // could not be clicked again.
                    .filter(|(_, r)| r.present)
                    .map(|(i, _)| i)
                    .filter(|i| {
                        let name = &self.rules[*i].name;
                        let q = self.search.trim().to_lowercase();
                        q.is_empty() || name.to_lowercase().contains(&q)
                    })
                    .collect();
                if shown.is_empty() {
                    continue;
                }
                any_shown = true;
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(heading.to_uppercase())
                            .size(10.5)
                            .strong()
                            .color(MUTED),
                    );
                    ui.add_space(8.0);
                    ui.label(RichText::new(t.rules_hint).size(10.0).color(MUTED));
                    ui.add_space(8.0);
                    // Whether a winapp2.ini was picked up, right where the rows are.
                    let (text, colour) = if self.rule_files.is_empty() {
                        (t.rule_chip_none.to_string(), MUTED)
                    } else {
                        (
                            t.rule_chip_files
                                .replace("{}", &self.rule_files.len().to_string()),
                            ACCENT2,
                        )
                    };
                    ui.label(RichText::new(text).size(10.0).color(colour));
                });
                ui.add_space(4.0);
                egui::Frame::default()
                    .fill(CARD)
                    .corner_radius(CornerRadius::ZERO)
                    .stroke(Stroke::new(1.0_f32, LINE))
                    .inner_margin(egui::Margin::symmetric(12, 4))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for i in shown {
                            self.rule_row(ui, i);
                        }
                    });
                ui.add_space(8.0);
                continue;
            }
            let shown: Vec<Group> = groups
                .into_iter()
                .filter(|g| group_matches(self.lang, *g, &self.search))
                .collect();
            if shown.is_empty() {
                continue;
            }
            any_shown = true;
            ui.add_space(6.0);
            ui.label(
                RichText::new(heading.to_uppercase())
                    .size(10.5)
                    .strong()
                    .color(MUTED),
            );
            ui.add_space(4.0);
            egui::Frame::default()
                .fill(CARD)
                .corner_radius(CornerRadius::ZERO)
                .stroke(Stroke::new(1.0_f32, LINE))
                .inner_margin(egui::Margin::symmetric(12, 4))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    for g in shown {
                        self.category_row(ui, g);
                    }
                });
            ui.add_space(8.0);
        }
        if !any_shown {
            ui.add_space(6.0);
            ui.label(RichText::new(t.no_match).size(12.0).color(MUTED));
        }
    }
    /// Guard the click, then hand the work to the worker thread.
    pub(crate) fn run_cleaner(&mut self) {
        let t = self.lang.t();
        if self.report.is_none() {
            self.set_status(t.need_analyze, true);
            return;
        }
        let groups = self.chosen_groups();
        let rules = self.chosen_rules();
        if groups.is_empty() && rules.is_empty() {
            self.set_status(t.tick_one, true);
        } else if self.ask_before {
            self.pending_clean = Some((groups, rules));
            self.confirm_clean = true;
        } else {
            self.do_clean(groups, rules);
        }
    }

    pub(crate) fn category_row(&mut self, ui: &mut egui::Ui, g: Group) {
        let t = self.lang.t();
        // The user's own folders only exist once one has been added.
        let available = if g == Group::Custom {
            !self.custom.is_empty()
        } else {
            crate::clean::available(g)
        };
        let idx = gindex(g);
        let (size_text, count) = match &self.report {
            // A row the scan hid (nothing found) can still be ticked — "select all"
            // skips hidden rows, but a stored tick may outlive a scan — so say 0
            // rather than leave the badge blank.
            Some(r) if r.group_count(g) > 0 || r.per_group.iter().any(|(k, _, _)| *k == g) => (
                crate::clean::human(r.group_size(g)),
                t.files.replace("{}", &r.group_count(g).to_string()),
            ),
            Some(_) => (
                crate::clean::human(0),
                t.files.replace("{}", &0.to_string()),
            ),
            None => (String::new(), String::new()),
        };
        let badge = if count.is_empty() {
            None
        } else {
            Some(format!("{}  ·  {}", size_text, count))
        };
        let note = if available {
            i18n::group_note(self.lang, g)
        } else {
            t.unavailable
        };
        let on = self.sel[idx] && available;
        if row(
            ui,
            on,
            available,
            i18n::group_label(self.lang, g),
            note,
            badge,
        ) && available
        {
            self.sel[idx] = !self.sel[idx];
            self.status.clear();
        }
    }

    /// One loaded application rule: name, note and what it found.
    pub(crate) fn rule_row(&mut self, ui: &mut egui::Ui, i: usize) {
        let t = self.lang.t();
        let Some(rule) = self.rules.get(i) else {
            return;
        };
        let available = rule.present;
        let (size_text, count) = match &self.report {
            Some(r) => {
                let (bytes, n) = r.rule_totals(i);
                if n > 0 {
                    (
                        crate::clean::human(bytes),
                        t.files.replace("{}", &n.to_string()),
                    )
                } else {
                    (String::new(), String::new())
                }
            }
            None => (String::new(), String::new()),
        };
        let badge = if count.is_empty() {
            None
        } else {
            Some(format!("{}  ·  {}", size_text, count))
        };
        // What this rule actually found is its best evidence, so say so when the
        // detection rests on something else: a leftover folder, or a shared key.
        let note = match (rule.warning.as_deref(), count.is_empty()) {
            (Some(w), _) => w,
            (None, false) => t.note_safe,
            (None, true) => t.rule_leftovers,
        };
        let on = self.rule_sel.get(i).copied().unwrap_or(false) && available;
        if row(ui, on, available, &rule.name, note, badge) && available {
            if let Some(slot) = self.rule_sel.get_mut(i) {
                *slot = !*slot;
            }
            self.status.clear();
        }
    }
    pub(crate) fn confirm_dialog(&mut self, ctx: &egui::Context) {
        let t = self.lang.t();
        let mut open = true;
        let mut answer: Option<bool> = None;
        egui::Window::new(t.sure_title)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_width(360.0);
                // How many boxes are ticked, not their names: a list long enough to
                // need scrolling would only repeat what the window behind it shows.
                let count = self
                    .pending_clean
                    .as_ref()
                    .map(|(gs, rs)| gs.len() + rs.len())
                    .unwrap_or(0);
                ui.label(
                    RichText::new(i18n::fill(t.will_delete, &[&count.to_string()]))
                        .size(12.5)
                        .color(TEXT),
                );
                if self.sel[gindex(Group::RecycleBin)] {
                    ui.add_space(6.0);
                    ui.label(RichText::new(t.bin_too).size(11.5).color(WARN));
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if primary_button(ui, t.delete, Vec2::new(110.0, 34.0)).clicked() {
                        answer = Some(true);
                    }
                    ui.add_space(8.0);
                    if ghost_button(ui, t.cancel, Vec2::new(110.0, 34.0)).clicked() {
                        answer = Some(false);
                    }
                });
            });
        match answer {
            Some(true) => {
                if let Some((groups, rules)) = self.pending_clean.take() {
                    self.do_clean(groups, rules);
                }
                self.confirm_clean = false;
            }
            Some(false) => {
                self.pending_clean = None;
                self.confirm_clean = false;
                self.set_status(t.nothing_deleted, false);
            }
            None => {
                if !open {
                    self.pending_clean = None;
                    self.confirm_clean = false;
                }
            }
        }
    }

    pub(crate) fn registry_page(&mut self, ui: &mut egui::Ui) {
        let t = self.lang.t();
        card(ui, |ui| {
            ui.label(
                RichText::new(t.what_to_check)
                    .size(13.0)
                    .strong()
                    .color(TEXT),
            );
            ui.add_space(6.0);
            ui.checkbox(&mut self.reg_checks[0], t.check_shortcuts);
            ui.checkbox(&mut self.reg_checks[1], t.check_startup);
            ui.checkbox(&mut self.reg_checks[2], t.check_uninstall);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if primary_button(ui, t.scan_for_issues, Vec2::new(160.0, 36.0)).clicked() {
                    self.start_reg_scan();
                }
                ui.add_space(10.0);
                if !self.reg_status.is_empty() {
                    ui.label(RichText::new(&self.reg_status).size(12.0).color(MUTED));
                }
            });
        });
        ui.add_space(12.0);

        if !self.reg_scanned {
            ui.label(RichText::new(t.nothing_scanned).size(12.0).color(MUTED));
            return;
        }
        if self.reg_issues.is_empty() {
            ui.label(RichText::new(t.no_problems).size(12.5).color(TEXT));
            return;
        }

        egui::Frame::default()
            .fill(CARD)
            .corner_radius(CornerRadius::ZERO)
            .stroke(Stroke::new(1.0_f32, LINE))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for i in 0..self.reg_issues.len() {
                            let issue = self.reg_issues[i].clone();
                            ui.horizontal(|ui| {
                                ui.checkbox(
                                    &mut self.reg_sel[i],
                                    RichText::new(&issue.label).size(12.5).color(TEXT),
                                );
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!(
                                            "{} · {} · {}",
                                            i18n::kind_label(self.lang, issue.kind),
                                            issue.source,
                                            i18n::detail_label(self.lang, &issue.detail)
                                        ))
                                        .size(11.0)
                                        .color(MUTED),
                                    )
                                    .wrap(),
                                );
                            });
                        }
                    });
            });
        ui.add_space(10.0);
        let fixable = self
            .reg_issues
            .iter()
            .enumerate()
            .filter(|(i, it)| self.reg_sel[*i] && it.fix.is_some())
            .count();
        ui.horizontal(|ui| {
            if primary_button(ui, t.fix_selected, Vec2::new(150.0, 36.0)).clicked() {
                let mut fixed = 0;
                let mut tried = 0;
                let mut done: Vec<usize> = Vec::new();
                for i in 0..self.reg_issues.len() {
                    if !self.reg_sel[i] {
                        continue;
                    }
                    let Some(fix) = self.reg_issues[i].fix.clone() else {
                        continue;
                    };
                    tried += 1;
                    let ok = match fix {
                        registry::Fix::Startup { hive, key, value } => {
                            registry::fix_startup(hive, &key, &value)
                        }
                        // A file, not a registry value: to the Recycle Bin, so a
                        // shortcut removed by mistake can be brought back.
                        registry::Fix::Shortcut { path } => clean::recycle(&path),
                    };
                    if ok {
                        fixed += 1;
                        done.push(i);
                    }
                }
                for i in done.into_iter().rev() {
                    self.reg_issues.remove(i);
                }
                self.reg_sel = vec![true; self.reg_issues.len()];
                self.reg_status = if fixed > 0 {
                    t.entries_removed.replace("{}", &fixed.to_string())
                } else if tried > 0 {
                    // HKLM needs elevation; say so instead of looking like a no-op.
                    t.cannot_remove.replace("{}", &tried.to_string())
                } else {
                    t.only_fixable.into()
                };
            }
            ui.add_space(10.0);
            ui.label(
                RichText::new(t.fixable_count.replace("{}", &fixable.to_string()))
                    .size(11.5)
                    .color(MUTED),
            );
        });
        ui.add_space(8.0);
        ui.label(RichText::new(t.readonly_hint).size(10.5).color(MUTED));
    }

    pub(crate) fn tools_page(&mut self, ui: &mut egui::Ui) {
        let t = self.lang.t();
        // Cards share one width, so a narrow window wraps them instead of clipping.
        let card_w = ui.available_width().clamp(200.0, 272.0);
        // Tall enough for a blurb that wraps to two lines above the "open" label.
        let card_h = 104.0;
        ui.horizontal_wrapped(|ui| {
            for tool in i18n::tools(self.lang) {
                let (title, blurb, kind) = (tool.title, tool.blurb, tool.kind);
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(card_w, card_h), Sense::click());
                let hover = resp.hovered();
                let radius = CornerRadius::ZERO;
                ui.painter()
                    .rect_filled(rect, radius, if hover { CARD_HOVER } else { CARD });
                ui.painter().rect_stroke(
                    rect,
                    radius,
                    Stroke::new(1.0_f32, if hover { ACCENT } else { LINE }),
                    egui::StrokeKind::Inside,
                );
                // a small square marker, no rounded tile
                let tile = Rect::from_min_size(rect.min + Vec2::new(16.0, 16.0), Vec2::splat(14.0));
                ui.painter().rect_filled(
                    tile,
                    CornerRadius::ZERO,
                    if hover { ACCENT } else { CARD_SOFT },
                );
                ui.painter().text(
                    rect.min + Vec2::new(38.0, 23.0),
                    egui::Align2::LEFT_CENTER,
                    title,
                    FontId::proportional(13.0),
                    TEXT,
                );
                let galley = ui.painter().layout(
                    blurb.to_owned(),
                    FontId::proportional(10.5),
                    MUTED,
                    rect.width() - 32.0,
                );
                ui.painter()
                    .galley(rect.min + Vec2::new(16.0, 44.0), galley, MUTED);
                ui.painter().text(
                    rect.left_bottom() + Vec2::new(16.0, -16.0),
                    egui::Align2::LEFT_BOTTOM,
                    t.open,
                    FontId::proportional(11.0),
                    if hover { TEXT } else { MUTED },
                );
                if resp.clicked() {
                    if let Some(target) = sysinfo::tool_target(kind) {
                        let _ = open_path(&target);
                        self.set_status(t.opened.replace("{}", &title.to_lowercase()), false);
                    } else {
                        self.set_status(
                            t.could_not_find.replace("{}", &title.to_lowercase()),
                            true,
                        );
                    }
                }
                ui.add_space(10.0);
            }
        });
        ui.add_space(14.0);
        card(ui, |ui| {
            ui.label(RichText::new(t.drive_space).size(13.0).strong().color(TEXT));
            ui.add_space(6.0);
            let total = self.sys.disk_total.max(1);
            let used = total.saturating_sub(self.sys.disk_free);
            let frac = used as f32 / total as f32;
            ui.add(
                egui::ProgressBar::new(frac)
                    .desired_width(ui.available_width() - 8.0)
                    .desired_height(14.0)
                    .text(
                        RichText::new(i18n::fill(
                            t.used_of,
                            &[&crate::clean::human(used), &crate::clean::human(total)],
                        ))
                        .size(10.5),
                    ),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    t.free
                        .replace("{}", &crate::clean::human(self.sys.disk_free)),
                )
                .size(11.0)
                .color(MUTED),
            );
        });
    }

    pub(crate) fn options_page(&mut self, ui: &mut egui::Ui) {
        let t = self.lang.t();
        card(ui, |ui| {
            ui.label(RichText::new(t.cleaning).size(13.0).strong().color(TEXT));
            ui.add_space(6.0);
            ui.checkbox(&mut self.ask_before, t.ask_before);
            ui.add_space(4.0);
            ui.label(RichText::new(t.ask_before_hint).size(10.5).color(MUTED));
            ui.add_space(8.0);
            ui.checkbox(&mut self.auto_analyze, t.auto_analyze);
        });
        ui.add_space(12.0);
        card(ui, |ui| {
            ui.label(RichText::new(t.language).size(13.0).strong().color(TEXT));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                for &lang in Lang::ALL.iter() {
                    if pill(ui, lang.label(), self.lang == lang).clicked() {
                        self.lang = lang;
                    }
                    ui.add_space(8.0);
                }
            });
            ui.add_space(6.0);
            ui.label(RichText::new(t.language_hint).size(10.5).color(MUTED));
        });
        ui.add_space(12.0);
        self.rule_card(ui);
        ui.add_space(12.0);
        card(ui, |ui| {
            ui.label(RichText::new(t.categories).size(13.0).strong().color(TEXT));
            ui.add_space(6.0);
            for &g in Group::ALL.iter() {
                let mut on = self.sel[gindex(g)];
                if ui
                    .checkbox(&mut on, i18n::group_label(self.lang, g))
                    .changed()
                {
                    self.sel[gindex(g)] = on;
                }
            }
            ui.add_space(8.0);
            ui.label(RichText::new(t.categories_hint).size(10.5).color(MUTED));
        });
        ui.add_space(12.0);
        self.folder_lists(ui);
    }

    /// Which rule files were picked up, so a winapp2.ini that is present is said
    /// to be present rather than silently used or silently ignored.
    fn rule_card(&mut self, ui: &mut egui::Ui) {
        let t = self.lang.t();
        card(ui, |ui| {
            ui.label(RichText::new(t.rule_files).size(13.0).strong().color(TEXT));
            ui.add_space(8.0);
            rule_file_line(
                ui,
                t.rule_builtin,
                t.rule_sections
                    .replace("{}", &self.builtin_rules.to_string()),
                MUTED,
            );
            for (path, sections) in self.rule_files.clone() {
                rule_file_line(
                    ui,
                    t.rule_extra,
                    t.rule_sections.replace("{}", &sections.to_string()),
                    ACCENT2,
                );
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(path.display().to_string())
                                .size(10.5)
                                .color(MUTED),
                        )
                        .truncate(),
                    );
                });
            }
            if self.rule_files.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new(t.rule_none).size(10.5).color(MUTED));
            }
            ui.add_space(8.0);
            ui.add(egui::Label::new(RichText::new(t.rule_hint).size(10.5).color(MUTED)).wrap());
        });
    }

    /// The two folder lists: places to empty, and places never to touch.
    fn folder_lists(&mut self, ui: &mut egui::Ui) {
        let t = self.lang.t();
        card(ui, |ui| {
            ui.label(
                RichText::new(t.custom_folders)
                    .size(13.0)
                    .strong()
                    .color(TEXT),
            );
            ui.add_space(4.0);
            ui.add(egui::Label::new(RichText::new(t.custom_hint).size(10.5).color(MUTED)).wrap());
            ui.add_space(8.0);
            let mut drop_custom: Option<usize> = None;
            for (i, p) in self.custom.iter().enumerate() {
                ui.horizontal(|ui| {
                    // The path can be long; it gets the row minus the button.
                    let w = (ui.available_width() - 90.0).max(80.0);
                    ui.add_sized(
                        Vec2::new(w, 20.0),
                        egui::Label::new(
                            RichText::new(p.display().to_string())
                                .size(11.0)
                                .color(TEXT),
                        )
                        .truncate(),
                    );
                    if ghost_button(ui, t.remove, Vec2::new(76.0, 24.0)).clicked() {
                        drop_custom = Some(i);
                    }
                });
            }
            if let Some(i) = drop_custom {
                self.custom.remove(i);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 90.0).max(80.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_custom)
                        .hint_text(t.path_hint)
                        .desired_width(w),
                );
                if ghost_button(ui, t.add, Vec2::new(76.0, 24.0)).clicked() {
                    self.add_folder(true);
                }
            });
        });
        ui.add_space(12.0);
        card(ui, |ui| {
            ui.label(
                RichText::new(t.exclude_folders)
                    .size(13.0)
                    .strong()
                    .color(TEXT),
            );
            ui.add_space(4.0);
            ui.add(egui::Label::new(RichText::new(t.exclude_hint).size(10.5).color(MUTED)).wrap());
            ui.add_space(8.0);
            let mut drop_ex: Option<usize> = None;
            for (i, p) in self.exclude.iter().enumerate() {
                ui.horizontal(|ui| {
                    let w = (ui.available_width() - 90.0).max(80.0);
                    ui.add_sized(
                        Vec2::new(w, 20.0),
                        egui::Label::new(
                            RichText::new(p.display().to_string())
                                .size(11.0)
                                .color(TEXT),
                        )
                        .truncate(),
                    );
                    if ghost_button(ui, t.remove, Vec2::new(76.0, 24.0)).clicked() {
                        drop_ex = Some(i);
                    }
                });
            }
            if let Some(i) = drop_ex {
                self.exclude.remove(i);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 90.0).max(80.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_exclude)
                        .hint_text(t.path_hint)
                        .desired_width(w),
                );
                if ghost_button(ui, t.add, Vec2::new(76.0, 24.0)).clicked() {
                    self.add_folder(false);
                }
            });
            ui.add_space(8.0);
            ui.label(
                RichText::new(t.exclusions_take_precedence)
                    .size(10.5)
                    .color(MUTED),
            );
        });
    }

    /// Adds what is typed into one of the folder boxes, after checking it is a real
    /// folder, not already listed, and not inside something excluded.
    fn add_folder(&mut self, to_custom: bool) {
        let t = self.lang.t();
        let typed = if to_custom {
            self.new_custom.trim().to_string()
        } else {
            self.new_exclude.trim().to_string()
        };
        if typed.is_empty() {
            return;
        }
        let path = PathBuf::from(&typed);
        if !path.is_dir() {
            self.set_status(t.folder_missing, true);
            return;
        }
        let list = if to_custom {
            &self.custom
        } else {
            &self.exclude
        };
        if list.iter().any(|p| clean::same_folder(p, &path)) {
            self.set_status(t.already_added, true);
            return;
        }
        if to_custom && self.exclude.iter().any(|e| clean::inside(&path, e)) {
            self.set_status(t.path_is_excluded, true);
            return;
        }
        if to_custom {
            self.custom.push(path);
            self.new_custom.clear();
        } else {
            self.exclude.push(path);
            self.new_exclude.clear();
        }
        self.status.clear();
    }

    pub(crate) fn info_page(&mut self, ui: &mut egui::Ui) {
        let t = self.lang.t();
        card(ui, |ui| {
            let rows: [(&str, String); 7] = [
                (
                    t.computer,
                    i18n::fill(t.signed_in, &[&self.sys.host, &self.sys.user]),
                ),
                (t.system, self.sys.os.clone()),
                (t.processor, self.sys.cpu.clone()),
                (
                    t.memory,
                    // Free memory is the part a cleaner can actually give back,
                    // so both numbers are shown, not just what is in use.
                    i18n::fill(
                        t.used_free_of,
                        &[
                            &crate::clean::human(self.sys.ram_used),
                            &crate::clean::human(self.sys.ram_free),
                            &crate::clean::human(self.sys.ram_total),
                        ],
                    ),
                ),
                (
                    t.free_disk_space,
                    i18n::fill(
                        t.of,
                        &[
                            &crate::clean::human(self.sys.disk_free),
                            &crate::clean::human(self.sys.disk_total),
                        ],
                    ),
                ),
                (
                    t.temp_folder,
                    std::env::var("TEMP").unwrap_or_else(|_| t.unknown.into()),
                ),
                (t.app, t.app_build.replace("{}", std::env::consts::ARCH)),
            ];
            let narrow = ui.available_width() < 420.0;
            for (label, value) in rows {
                if narrow {
                    // Too little width for two columns: label on top, value beneath.
                    ui.label(RichText::new(label).size(11.5).color(MUTED));
                    ui.add(egui::Label::new(RichText::new(value).size(12.0).color(TEXT)).wrap());
                } else {
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            Vec2::new(150.0, 18.0),
                            egui::Label::new(RichText::new(label).size(11.5).color(MUTED)),
                        );
                        ui.add(
                            egui::Label::new(RichText::new(value).size(12.0).color(TEXT)).wrap(),
                        );
                    });
                }
                ui.add_space(4.0);
            }
        });
        ui.add_space(10.0);
        if ghost_button(ui, t.refresh, Vec2::new(110.0, 32.0)).clicked() {
            self.sys = sysinfo::collect();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_moves_from_a_to_b() {
        let a = Color32::from_rgb(10, 20, 30);
        let b = Color32::from_rgb(110, 220, 250);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
        assert_eq!(mix(a, b, 0.5), Color32::from_rgb(60, 120, 140));
        // out-of-range factors clamp instead of overflowing a channel
        assert_eq!(mix(a, b, -1.0), a);
        assert_eq!(mix(a, b, 2.0), b);
    }

    #[test]
    fn fade_is_finished_without_a_start_time() {
        assert_eq!(fade_in(None), 1.0);
    }
}
