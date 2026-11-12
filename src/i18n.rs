//! Every word the interface says, in English and Indonesian.
//!
//! One `Text` value per language, all fields `&'static str`, so a missing
//! translation is a compile error rather than a blank label. Strings that take
//! values keep plain `{}` placeholders and are filled with `format!`.

use crate::clean::{Group, Tab};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Lang {
    #[default]
    En,
    Id,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Id];

    /// The name of the language, written in that language.
    pub fn label(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Id => "Indonesia",
        }
    }

    pub fn t(self) -> &'static Text {
        match self {
            Lang::En => &EN,
            Lang::Id => &ID,
        }
    }
}

pub struct Text {
    // sidebar
    pub workspace: &'static str,
    pub run_as_admin: &'static str,
    pub dry_run: &'static str,
    pub collapse: &'static str,
    pub expand: &'static str,

    // page titles and subtitles
    pub page_cleaner: &'static str,
    pub page_registry: &'static str,
    pub page_tools: &'static str,
    pub page_options: &'static str,
    pub page_info: &'static str,
    pub sub_cleaner: &'static str,
    pub sub_registry: &'static str,
    pub sub_tools: &'static str,
    pub sub_options: &'static str,
    pub sub_info: &'static str,

    // status bar
    pub analyzing: &'static str,
    pub looking_issues: &'static str,
    pub analysis_just_finished: &'static str,
    pub last_analysis_secs: &'static str,
    pub last_analysis_mins: &'static str,
    pub ready: &'static str,
    pub free_of: &'static str,
    /// "{} used \u{00B7} {} free of {}" for memory.
    pub used_free_of: &'static str,

    // cleaner page
    pub run_cleaner: &'static str,
    /// The empty state, and the two scan buttons.
    pub scan_now: &'static str,
    pub scan_again: &'static str,
    pub empty_title: &'static str,
    pub empty_body: &'static str,
    pub scanning_title: &'static str,
    pub scanning_body: &'static str,
    /// The sticky footer's two lines.
    pub selected_total: &'static str,
    pub nothing_selected: &'static str,
    pub found_total: &'static str,
    pub need_analyze: &'static str,
    pub tick_one: &'static str,
    pub files: &'static str,
    pub unavailable: &'static str,
    pub tab_windows: &'static str,
    pub tab_apps: &'static str,
    pub tab_advanced: &'static str,
    pub head_windows_explorer: &'static str,
    pub head_system: &'static str,
    pub head_internet: &'static str,
    pub head_apps: &'static str,
    pub head_downloads: &'static str,
    pub head_windows: &'static str,
    pub head_advanced: &'static str,
    pub head_app_rules: &'static str,
    pub rules_hint: &'static str,
    /// The Options card about rule files.
    pub rule_files: &'static str,
    pub rule_builtin: &'static str,
    pub rule_extra: &'static str,
    pub rule_sections: &'static str,
    pub rule_none: &'static str,
    pub rule_hint: &'static str,
    /// The one-line source chip above the program rows.
    pub rule_chip_none: &'static str,
    pub rule_chip_files: &'static str,
    pub sure_title: &'static str,
    pub will_delete: &'static str,
    pub bin_too: &'static str,
    pub delete: &'static str,
    pub cancel: &'static str,
    pub nothing_deleted: &'static str,

    // categories
    pub group_windows_temp: &'static str,
    pub group_user_temp: &'static str,
    pub group_recycle_bin: &'static str,
    pub group_thumbnails: &'static str,
    pub group_prefetch: &'static str,
    pub group_font_cache: &'static str,
    pub group_shader_cache: &'static str,
    pub group_wu_cache: &'static str,
    pub group_do_files: &'static str,
    pub group_error_reports: &'static str,
    pub group_browser_cache: &'static str,
    pub group_browser_cookies: &'static str,
    pub group_app_caches: &'static str,
    pub group_recent_docs: &'static str,
    pub group_logs: &'static str,
    pub group_memory_dumps: &'static str,
    pub group_downloads: &'static str,
    pub group_windows_old: &'static str,
    pub group_custom: &'static str,
    pub group_app_rule: &'static str,
    pub note_recycle: &'static str,
    pub note_recent: &'static str,
    pub note_downloads: &'static str,
    pub note_wu: &'static str,
    pub note_safe: &'static str,
    /// Shown instead when a rule detected its program but found nothing: the
    /// detection then rests on a leftover folder or a shared registry key.
    pub rule_leftovers: &'static str,
    pub note_prefetch: &'static str,
    pub note_cookies: &'static str,
    pub note_windows_old: &'static str,
    pub note_custom: &'static str,
    pub note_browser: &'static str,

    // registry page
    pub what_to_check: &'static str,
    pub check_shortcuts: &'static str,
    pub check_startup: &'static str,
    pub check_uninstall: &'static str,
    pub scan_for_issues: &'static str,
    pub nothing_scanned: &'static str,
    pub no_problems: &'static str,
    pub fix_selected: &'static str,
    pub entries_removed: &'static str,
    pub cannot_remove: &'static str,
    pub only_fixable: &'static str,
    pub fixable_count: &'static str,
    pub readonly_hint: &'static str,
    pub no_issues_found: &'static str,
    pub issues_found: &'static str,
    pub scan_stopped: &'static str,
    pub kind_shortcuts: &'static str,
    pub kind_startup: &'static str,
    pub kind_uninstall: &'static str,
    pub points_at: &'static str,
    pub program_gone: &'static str,
    pub folder_gone: &'static str,

    // run results
    pub tick_first: &'static str,
    pub analysis_complete: &'static str,
    pub analysis_stopped: &'static str,
    pub removing: &'static str,
    pub clean_stopped: &'static str,
    pub dry_run_done: &'static str,
    pub done_clean: &'static str,
    pub partial_clean: &'static str,
    pub recycled_suffix: &'static str,

    // tools page
    pub tool_uninstall: &'static str,
    pub tool_uninstall_blurb: &'static str,
    pub tool_startup: &'static str,
    pub tool_startup_blurb: &'static str,
    pub tool_recycle: &'static str,
    pub tool_recycle_blurb: &'static str,
    pub tool_temp: &'static str,
    pub tool_temp_blurb: &'static str,
    pub tool_downloads: &'static str,
    pub tool_downloads_blurb: &'static str,
    pub tool_services: &'static str,
    pub tool_services_blurb: &'static str,
    pub open: &'static str,
    pub opened: &'static str,
    pub could_not_find: &'static str,
    pub drive_space: &'static str,
    pub used_of: &'static str,
    pub free: &'static str,

    // options page
    pub cleaning: &'static str,
    pub ask_before: &'static str,
    pub ask_before_hint: &'static str,
    pub auto_analyze: &'static str,
    pub categories: &'static str,
    pub categories_hint: &'static str,
    pub search_hint: &'static str,
    pub select_all: &'static str,
    pub select_none: &'static str,
    pub no_match: &'static str,
    pub custom_folders: &'static str,
    pub custom_hint: &'static str,
    pub exclude_folders: &'static str,
    pub exclude_hint: &'static str,
    pub path_hint: &'static str,
    pub add: &'static str,
    pub remove: &'static str,
    pub folder_missing: &'static str,
    pub already_added: &'static str,
    pub path_is_excluded: &'static str,
    pub exclusions_take_precedence: &'static str,
    pub language: &'static str,
    pub language_hint: &'static str,

    // system information
    pub computer: &'static str,
    pub system: &'static str,
    pub processor: &'static str,
    pub memory: &'static str,
    pub free_disk_space: &'static str,
    pub temp_folder: &'static str,
    pub app: &'static str,
    pub signed_in: &'static str,
    pub unknown: &'static str,
    pub app_build: &'static str,
    pub refresh: &'static str,
    pub of: &'static str,
}

const EN: Text = Text {
    workspace: "WORKSPACE",
    run_as_admin: "Run as administrator",
    dry_run: "DRY RUN",
    collapse: "Hide the sidebar",
    expand: "Show the sidebar",

    page_cleaner: "Cleaner",
    page_registry: "Registry",
    page_tools: "Tools",
    page_options: "Options",
    page_info: "System Information",
    sub_cleaner: "Remove temporary files and other things you no longer need.",
    sub_registry: "Look for entries that point at programs and files which are gone.",
    sub_tools: "Shortcuts to the windows you would otherwise hunt for.",
    sub_options: "How the cleaner behaves.",
    sub_info: "What this computer is.",

    analyzing: "Analyzing…",
    looking_issues: "Looking for issues…",
    analysis_just_finished: "Analysis just finished.",
    last_analysis_secs: "Last analysis {} seconds ago.",
    last_analysis_mins: "Last analysis {} minutes ago.",
    ready: "Ready.",
    free_of: "{} free of {}",
    used_free_of: "{} used \u{00B7} {} free of {}",
    run_cleaner: "Run Cleaner",
    scan_now: "Scan now",
    scan_again: "Scan again",
    empty_title: "See what can go",
    empty_body: "One scan finds the temporary files, caches and leftovers on this PC. Nothing is removed until you tick it and run the cleaner.",
    scanning_title: "Looking through your files",
    scanning_body: "Reading the folders this PC collects junk in. The window stays usable while it runs.",
    selected_total: "Selected: {}",
    nothing_selected: "Nothing ticked yet",
    found_total: "{} files found \u{00B7} {}",
    need_analyze: "Press Analyze first, so the cleaner knows what to remove.",
    tick_one: "Tick at least one category.",
    files: "{} files",
    unavailable: "not found on this computer",
    tab_windows: "Windows",
    tab_apps: "Applications",
    tab_advanced: "Advanced",
    head_windows_explorer: "Windows Explorer",
    head_system: "System",
    head_internet: "Internet",
    head_apps: "Applications",
    head_downloads: "File downloads",
    head_windows: "Windows",
    head_advanced: "Extra places",
    head_app_rules: "Programs found on this PC",
    rules_hint: "Built-in list first, then winapp2.ini.",
    rule_files: "Application rule files",
    rule_builtin: "Built into the program",
    rule_extra: "Loaded from a file",
    rule_sections: "{} sections",
    rule_none: "No winapp2.ini found: only the built-in list is used.",
    rule_hint: "Read once, at start: the built-in list, then winapp2.ini beside the program or in %APPDATA%\\BCleaner. A later file replaces a section of the same name.",
    rule_chip_none: "built-in rules only",
    rule_chip_files: "winapp2.ini: {} file(s) loaded",
    sure_title: "Are you sure?",
    will_delete: "{} selected items will be deleted for good:",
    bin_too: "The Recycle Bin will be emptied too.",
    delete: "Delete",
    cancel: "Cancel",
    nothing_deleted: "Nothing was deleted.",

    group_windows_temp: "Temporary files (system)",
    group_user_temp: "Temporary files (my account)",
    group_recycle_bin: "Recycle Bin",
    group_thumbnails: "Thumbnail cache",
    group_prefetch: "Windows prefetch data",
    group_font_cache: "Font cache",
    group_shader_cache: "DirectX shader cache",
    group_wu_cache: "Windows Update leftovers",
    group_do_files: "Delivery Optimization files",
    group_error_reports: "Error reports and crash dumps",
    group_browser_cache: "Browsers: internet cache",
    group_browser_cookies: "Browsers: cookies and sign-ins",
    group_app_caches: "App caches (Teams, Discord, Slack, Store)",
    group_recent_docs: "Recent documents list",
    group_logs: "Windows log files",
    group_memory_dumps: "Memory dumps",
    group_downloads: "Downloaded installers (over a week old)",
    group_windows_old: "Previous Windows installation",
    group_custom: "My own folders",
    group_app_rule: "Application rule",
    note_recycle: "Permanently deletes everything in the Recycle Bin.",
    note_recent: "Clears the lists of recently opened files.",
    note_downloads: "Only .exe/.msi/.zip/.7z files older than 7 days.",
    note_wu: "May slow the next Windows Update.",
    note_safe: "Safe to remove.",
    rule_leftovers: "Program not found; only leftover folders matched.",
    note_prefetch: "Apps open a little slower for a while afterwards.",
    note_cookies: "Signs you out of every website. Browsers must be closed.",
    note_windows_old: "Cannot be undone. Windows will not roll back after this.",
    note_custom: "The folders stay; only what is inside them goes.",
    note_browser: "Close the browser first, or the files are locked.",

    what_to_check: "What to check",
    check_shortcuts: "Shortcuts in the Start menu and on the Desktop",
    check_startup: "Programs told to start with Windows",
    check_uninstall: "Installed programs whose folder is gone",
    scan_for_issues: "Scan for Issues",
    nothing_scanned: "Nothing scanned yet.",
    no_problems: "No problems found. Your shortcuts and startup list look healthy.",
    fix_selected: "Fix Selected",
    entries_removed: "{} entries removed.",
    cannot_remove: "Could not remove {} entries. A startup entry under HKLM needs administrator rights.",
    only_fixable: "Only startup entries and shortcuts can be removed automatically; the rest are listed for you to look at.",
    fixable_count: "{} of these can be removed here",
    readonly_hint: "Uninstall entries are shown read-only: the key also holds the repair and uninstall commands, so open the folder and decide yourself.",
    no_issues_found: "No issues found.",
    issues_found: "{} issues found.",
    scan_stopped: "The scan stopped unexpectedly.",
    kind_shortcuts: "Shortcuts",
    kind_startup: "Startup programs",
    kind_uninstall: "Uninstall entries",
    points_at: "points at {}",
    program_gone: "program is gone: {}",
    folder_gone: "folder is gone: {}",

    tick_first: "Tick at least one category first.",
    analysis_complete: "Analysis complete: {} files, {} could be removed.",
    analysis_stopped: "The analysis stopped unexpectedly.",
    removing: "Removing…",
    clean_stopped: "The cleaner stopped unexpectedly. Some items may already be gone.",
    dry_run_done: "Dry run: {} items, {} would be removed. Nothing was touched.",
    done_clean: "Done: {} removed from {} items.{}",
    partial_clean: "{} freed. {} items could not be removed (in use or protected).{}",
    recycled_suffix: " {} went to the Recycle Bin.",

    tool_uninstall: "Uninstall programs",
    tool_uninstall_blurb: "The classic list of everything installed on this PC.",
    tool_startup: "Startup apps",
    tool_startup_blurb: "The Startup folder, where shortcuts run at sign-in.",
    tool_recycle: "Recycle Bin",
    tool_recycle_blurb: "Open the bin to restore or empty it yourself.",
    tool_temp: "Temporary folder",
    tool_temp_blurb: "Where programs drop files they meant to delete.",
    tool_downloads: "Downloads",
    tool_downloads_blurb: "Your downloads folder, untouched and in one click.",
    tool_services: "Windows services",
    tool_services_blurb: "The list of background services.",
    open: "Open",
    opened: "Opened {}.",
    could_not_find: "Could not find {}.",
    drive_space: "Drive space",
    used_of: "{} used of {}",
    free: "{} free",

    cleaning: "Cleaning",
    ask_before: "Ask before deleting anything",
    ask_before_hint: "When off, Run Cleaner deletes straight away.",
    auto_analyze: "Analyze when the app starts",
    categories: "Categories",
    categories_hint: "These ticks are the same ones used on the Cleaner page.",
    search_hint: "Search categories",
    select_all: "Select all",
    select_none: "Select none",
    no_match: "Nothing on this tab matches the search.",
    custom_folders: "My own folders",
    custom_hint: "Extra folders to empty, one path per line. The folders stay; only what is inside them is removed.",
    exclude_folders: "Never touch these folders",
    exclude_hint: "Anything inside these folders is left alone, even when a category above would have cleaned it.",
    path_hint: "C:\\Folder\\To\\Clean",
    add: "Add",
    remove: "Remove",
    folder_missing: "No folder exists at that path.",
    already_added: "That folder is already on the list.",
    path_is_excluded: "That folder is inside one of the exclude folders.",
    exclusions_take_precedence: "Excluded folders are never cleaned, whatever else is ticked.",
    language: "Language",
    language_hint: "The interface switches as soon as you pick one.",

    computer: "Computer",
    system: "System",
    processor: "Processor",
    memory: "Memory",
    free_disk_space: "Free disk space",
    temp_folder: "Temporary folder",
    app: "App",
    signed_in: "{} (signed in as {})",
    unknown: "unknown",
    app_build: "Cleaner 1.0, built with Rust, running on {}",
    refresh: "Refresh",
    of: "{} of {}",
};

const ID: Text = Text {
    workspace: "RUANG KERJA",
    run_as_admin: "Jalankan sebagai administrator",
    dry_run: "UJI COBA",
    collapse: "Sembunyikan sidebar",
    expand: "Tampilkan sidebar",

    page_cleaner: "Pembersih",
    page_registry: "Registry",
    page_tools: "Alat",
    page_options: "Opsi",
    page_info: "Informasi Sistem",
    sub_cleaner: "Hapus file sementara dan hal lain yang tidak diperlukan lagi.",
    sub_registry: "Cari entri yang menunjuk ke program dan file yang sudah hilang.",
    sub_tools: "Pintasan ke jendela yang biasanya dicari-cari.",
    sub_options: "Cara kerja pembersih.",
    sub_info: "Tentang komputer ini.",

    analyzing: "Menganalisis…",
    looking_issues: "Mencari masalah…",
    analysis_just_finished: "Analisis baru saja selesai.",
    last_analysis_secs: "Analisis terakhir {} detik lalu.",
    last_analysis_mins: "Analisis terakhir {} menit lalu.",
    ready: "Siap.",
    free_of: "{} kosong dari {}",
    used_free_of: "{} terpakai \u{00B7} {} kosong dari {}",

    run_cleaner: "Jalankan Pembersih",
    scan_now: "Pindai sekarang",
    scan_again: "Pindai ulang",
    empty_title: "Lihat apa yang bisa dibuang",
    empty_body: "Satu pemindaian menemukan file sementara, cache, dan sisa-sisa di PC ini. Tidak ada yang dihapus sampai kamu mencentangnya dan menjalankan pembersih.",
    scanning_title: "Menelusuri file kamu",
    scanning_body: "Membaca folder tempat PC ini menumpuk sampah. Jendela tetap bisa dipakai selama proses berjalan.",
    selected_total: "Dipilih: {}",
    nothing_selected: "Belum ada yang dicentang",
    found_total: "{} file ditemukan \u{00B7} {}",
    need_analyze: "Tekan Analisis dulu, supaya pembersih tahu apa yang harus dihapus.",
    tick_one: "Centang minimal satu kategori.",
    files: "{} file",
    unavailable: "tidak ditemukan di komputer ini",
    tab_windows: "Windows",
    tab_apps: "Aplikasi",
    tab_advanced: "Lanjutan",
    head_windows_explorer: "Windows Explorer",
    head_system: "Sistem",
    head_internet: "Internet",
    head_apps: "Aplikasi",
    head_downloads: "Unduhan file",
    head_windows: "Windows",
    head_advanced: "Tempat tambahan",
    head_app_rules: "Program yang ada di PC ini",
    rules_hint: "Daftar bawaan dulu, lalu winapp2.ini.",
    rule_files: "File aturan aplikasi",
    rule_builtin: "Menyatu di dalam program",
    rule_extra: "Dibaca dari file",
    rule_sections: "{} section",
    rule_none: "winapp2.ini tidak ditemukan: hanya daftar bawaan yang dipakai.",
    rule_hint: "Dibaca sekali saat mulai: daftar bawaan, lalu winapp2.ini di samping program atau di %APPDATA%\\BCleaner. File yang belakangan menggantikan section dengan nama sama.",
    rule_chip_none: "hanya aturan bawaan",
    rule_chip_files: "winapp2.ini: {} file terbaca",
    sure_title: "Yakin?",
    will_delete: "{} kategori dipilih akan dihapus permanen:",
    bin_too: "Recycle Bin juga akan dikosongkan.",
    delete: "Hapus",
    cancel: "Batal",
    nothing_deleted: "Tidak ada yang dihapus.",

    group_windows_temp: "File sementara (sistem)",
    group_user_temp: "File sementara (akun saya)",
    group_recycle_bin: "Recycle Bin",
    group_thumbnails: "Cache thumbnail",
    group_prefetch: "Data prefetch Windows",
    group_font_cache: "Cache font",
    group_shader_cache: "Cache shader DirectX",
    group_wu_cache: "Sisa Windows Update",
    group_do_files: "File Delivery Optimization",
    group_error_reports: "Laporan error dan crash dump",
    group_browser_cache: "Browser: cache internet",
    group_browser_cookies: "Browser: cookie dan sesi masuk",
    group_app_caches: "Cache aplikasi (Teams, Discord, Slack, Store)",
    group_recent_docs: "Daftar dokumen terbaru",
    group_logs: "File log Windows",
    group_memory_dumps: "Memory dump",
    group_downloads: "Installer unduhan (lebih dari seminggu)",
    group_windows_old: "Instalasi Windows sebelumnya",
    group_custom: "Folder saya sendiri",
    group_app_rule: "Aturan aplikasi",
    note_recycle: "Menghapus permanen semua isi Recycle Bin.",
    note_recent: "Membersihkan daftar file yang baru dibuka.",
    note_downloads: "Hanya .exe/.msi/.zip/.7z yang lebih tua dari 7 hari.",
    note_wu: "Bisa memperlambat Windows Update berikutnya.",
    note_safe: "Aman dihapus.",
    rule_leftovers: "Programnya tidak ketemu; cuma sisa folder yang cocok.",
    note_prefetch: "Aplikasi jadi sedikit lambat dibuka untuk sementara.",
    note_cookies: "Mengeluarkan kamu dari semua situs. Browser harus ditutup dulu.",
    note_windows_old: "Tidak bisa dibatalkan. Windows tidak bisa rollback setelah ini.",
    note_custom: "Foldernya tetap ada; hanya isinya yang dihapus.",
    note_browser: "Tutup browser dulu, kalau tidak filenya terkunci.",

    what_to_check: "Yang diperiksa",
    check_shortcuts: "Shortcut di Start menu dan Desktop",
    check_startup: "Program yang dijalankan saat Windows mulai",
    check_uninstall: "Program terpasang yang foldernya sudah hilang",
    scan_for_issues: "Cari Masalah",
    nothing_scanned: "Belum ada yang dipindai.",
    no_problems: "Tidak ada masalah. Shortcut dan daftar startup kamu terlihat sehat.",
    fix_selected: "Perbaiki yang Dipilih",
    entries_removed: "{} entri dihapus.",
    cannot_remove: "Tidak bisa menghapus {} entri. Entri startup di HKLM butuh hak administrator.",
    only_fixable: "Hanya entri startup dan shortcut yang bisa dihapus otomatis; sisanya hanya ditampilkan.",
    fixable_count: "{} dari ini bisa dihapus di sini",
    readonly_hint: "Entri uninstall hanya ditampilkan: key-nya juga menyimpan perintah repair dan uninstall, jadi buka foldernya dan putuskan sendiri.",
    no_issues_found: "Tidak ada masalah ditemukan.",
    issues_found: "{} masalah ditemukan.",
    scan_stopped: "Pemindaian berhenti tak terduga.",
    kind_shortcuts: "Shortcut",
    kind_startup: "Program startup",
    kind_uninstall: "Entri uninstall",
    points_at: "menunjuk ke {}",
    program_gone: "programnya hilang: {}",
    folder_gone: "foldernya hilang: {}",

    tick_first: "Centang minimal satu kategori dulu.",
    analysis_complete: "Analisis selesai: {} file, {} bisa dihapus.",
    analysis_stopped: "Analisis berhenti tak terduga.",
    removing: "Menghapus…",
    clean_stopped: "Pembersih berhenti tak terduga. Sebagian item mungkin sudah terhapus.",
    dry_run_done: "Uji coba: {} item, {} akan dihapus. Tidak ada yang disentuh.",
    done_clean: "Selesai: {} dihapus dari {} item.{}",
    partial_clean: "{} dibebaskan. {} item tidak bisa dihapus (sedang dipakai atau dilindungi).{}",
    recycled_suffix: " {} dipindah ke Recycle Bin.",

    tool_uninstall: "Uninstall program",
    tool_uninstall_blurb: "Daftar klasik semua yang terpasang di PC ini.",
    tool_startup: "Aplikasi startup",
    tool_startup_blurb: "Folder Startup, tempat shortcut berjalan saat login.",
    tool_recycle: "Recycle Bin",
    tool_recycle_blurb: "Buka untuk memulihkan atau mengosongkan sendiri.",
    tool_temp: "Folder sementara",
    tool_temp_blurb: "Tempat program menaruh file yang seharusnya dihapus.",
    tool_downloads: "Unduhan",
    tool_downloads_blurb: "Folder unduhan kamu, utuh dan sekali klik.",
    tool_services: "Layanan Windows",
    tool_services_blurb: "Daftar layanan latar belakang.",
    open: "Buka",
    opened: "Membuka {}.",
    could_not_find: "Tidak bisa menemukan {}.",
    drive_space: "Ruang disk",
    used_of: "{} terpakai dari {}",
    free: "{} kosong",

    cleaning: "Pembersihan",
    ask_before: "Tanya dulu sebelum menghapus",
    ask_before_hint: "Kalau dimatikan, Jalankan Pembersih langsung menghapus.",
    auto_analyze: "Analisis saat aplikasi dibuka",
    categories: "Kategori",
    categories_hint: "Centang ini sama dengan yang dipakai di halaman Pembersih.",
    search_hint: "Cari kategori",
    select_all: "Pilih semua",
    select_none: "Kosongkan pilihan",
    no_match: "Tidak ada di tab ini yang cocok dengan pencarian.",
    custom_folders: "Folder saya sendiri",
    custom_hint: "Folder tambahan untuk dikosongkan, satu path per baris. Foldernya tetap ada; hanya isinya yang dihapus.",
    exclude_folders: "Jangan sentuh folder ini",
    exclude_hint: "Apa pun di dalam folder ini dibiarkan, walau kategori di atas seharusnya membersihkannya.",
    path_hint: "C:\\Folder\\Yang\\Dibersihkan",
    add: "Tambah",
    remove: "Hapus",
    folder_missing: "Tidak ada folder di path itu.",
    already_added: "Folder itu sudah ada di daftar.",
    path_is_excluded: "Folder itu ada di dalam salah satu folder pengecualian.",
    exclusions_take_precedence: "Folder yang dikecualikan tidak pernah dibersihkan, apa pun yang dicentang.",
    language: "Bahasa",
    language_hint: "Tampilan berganti begitu kamu memilih.",

    computer: "Komputer",
    system: "Sistem",
    processor: "Prosesor",
    memory: "Memori",
    free_disk_space: "Ruang disk kosong",
    temp_folder: "Folder sementara",
    app: "Aplikasi",
    signed_in: "{} (masuk sebagai {})",
    unknown: "tidak diketahui",
    app_build: "Cleaner 1.0, dibuat dengan Rust, berjalan di {}",
    refresh: "Segarkan",
    of: "{} dari {}",
};

pub fn group_label(lang: Lang, g: Group) -> &'static str {
    let t = lang.t();
    match g {
        Group::WindowsTemp => t.group_windows_temp,
        Group::UserTemp => t.group_user_temp,
        Group::RecycleBin => t.group_recycle_bin,
        Group::ThumbnailCache => t.group_thumbnails,
        Group::Prefetch => t.group_prefetch,
        Group::FontCache => t.group_font_cache,
        Group::ShaderCache => t.group_shader_cache,
        Group::WindowsUpdateCache => t.group_wu_cache,
        Group::DeliveryOptimization => t.group_do_files,
        Group::ErrorReports => t.group_error_reports,
        Group::BrowserCache => t.group_browser_cache,
        Group::BrowserCookies => t.group_browser_cookies,
        Group::AppCaches => t.group_app_caches,
        Group::RecentDocs => t.group_recent_docs,
        Group::WindowsLogs => t.group_logs,
        Group::MemoryDumps => t.group_memory_dumps,
        Group::Downloads => t.group_downloads,
        Group::WindowsOld => t.group_windows_old,
        Group::Custom => t.group_custom,
        Group::AppRule => t.group_app_rule,
    }
}

pub fn group_note(lang: Lang, g: Group) -> &'static str {
    let t = lang.t();
    match g {
        Group::RecycleBin => t.note_recycle,
        Group::RecentDocs => t.note_recent,
        Group::Downloads => t.note_downloads,
        Group::WindowsUpdateCache | Group::DeliveryOptimization => t.note_wu,
        Group::Prefetch => t.note_prefetch,
        Group::BrowserCookies => t.note_cookies,
        Group::WindowsOld => t.note_windows_old,
        Group::Custom => t.note_custom,
        Group::BrowserCache | Group::AppCaches | Group::ShaderCache => t.note_browser,
        _ => t.note_safe,
    }
}

pub fn tab_label(lang: Lang, tab: Tab) -> &'static str {
    let t = lang.t();
    match tab {
        Tab::Windows => t.tab_windows,
        Tab::Applications => t.tab_apps,
        Tab::Advanced => t.tab_advanced,
    }
}

pub fn kind_label(lang: Lang, kind: crate::registry::Kind) -> &'static str {
    let t = lang.t();
    match kind {
        crate::registry::Kind::Shortcut => t.kind_shortcuts,
        crate::registry::Kind::Startup => t.kind_startup,
        crate::registry::Kind::Uninstall => t.kind_uninstall,
    }
}

pub fn detail_label(lang: Lang, detail: &crate::registry::Detail) -> String {
    let t = lang.t();
    match detail {
        crate::registry::Detail::PointsAt(s) => fill(t.points_at, &[s]),
        crate::registry::Detail::ProgramGone(s) => fill(t.program_gone, &[s]),
        crate::registry::Detail::FolderGone(s) => fill(t.folder_gone, &[s]),
    }
}

/// Fills the `{}` placeholders of a sentence in order.
///
/// `str::replace` replaces *every* `{}` at once, so `"{} of {}"` became
/// "1 of 1": the second value never made it in. Every sentence with more than
/// one value goes through here instead.
pub fn fill(template: &str, values: &[&str]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    let mut next = 0;
    while let Some(at) = rest.find("{}") {
        out.push_str(&rest[..at]);
        out.push_str(values.get(next).copied().unwrap_or(""));
        rest = &rest[at + 2..];
        next += 1;
    }
    out.push_str(rest);
    out
}

/// One card on the Tools page. `kind` is the key `sysinfo::tool_target` knows.
pub struct Tool {
    pub title: &'static str,
    pub blurb: &'static str,
    pub kind: &'static str,
}

pub fn tools(lang: Lang) -> [Tool; 6] {
    let t = lang.t();
    [
        Tool {
            title: t.tool_uninstall,
            blurb: t.tool_uninstall_blurb,
            kind: "uninstall",
        },
        Tool {
            title: t.tool_startup,
            blurb: t.tool_startup_blurb,
            kind: "startup",
        },
        Tool {
            title: t.tool_recycle,
            blurb: t.tool_recycle_blurb,
            kind: "recycle",
        },
        Tool {
            title: t.tool_temp,
            blurb: t.tool_temp_blurb,
            kind: "temp",
        },
        Tool {
            title: t.tool_downloads,
            blurb: t.tool_downloads_blurb,
            kind: "downloads",
        },
        Tool {
            title: t.tool_services,
            blurb: t.tool_services_blurb,
            kind: "services",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Detail;

    #[test]
    fn each_language_has_its_own_words() {
        assert_ne!(EN.page_cleaner, ID.page_cleaner);
        assert_ne!(EN.only_fixable, ID.only_fixable);
        assert_ne!(EN.language_hint, ID.language_hint);
        assert_eq!(ID.language, "Bahasa");
        assert_eq!(EN.language, "Language");
    }

    #[test]
    fn details_are_filled_in_the_right_language() {
        assert_eq!(
            detail_label(Lang::En, &Detail::PointsAt("C:\\gone.lnk".into())),
            "points at C:\\gone.lnk"
        );
        assert_eq!(
            detail_label(Lang::Id, &Detail::ProgramGone("x.exe".into())),
            "programnya hilang: x.exe"
        );
        assert_eq!(
            detail_label(Lang::Id, &Detail::FolderGone("D:\\a".into())),
            "foldernya hilang: D:\\a"
        );
    }

    #[test]
    fn placeholders_are_filled_in_order() {
        assert_eq!(fill("{} of {}", &["1", "2"]), "1 of 2");
        assert_eq!(
            fill("{} \u{00B7} {} of {}", &["a", "b", "c"]),
            "a \u{00B7} b of c"
        );
        // a repeated value only goes into its own placeholder
        assert_eq!(fill("{} free of {}", &["7", "7"]), "7 free of 7");
        assert_eq!(fill("no holes", &["x"]), "no holes");
        // more holes than values leaves the extra ones empty, never panics
        assert_eq!(fill("{} {}", &["only"]), "only ");
    }

    #[test]
    fn every_group_has_a_name_and_a_note() {
        for g in Group::ALL {
            assert!(!group_label(Lang::En, g).is_empty());
            assert!(!group_label(Lang::Id, g).is_empty());
            assert!(!group_note(Lang::En, g).is_empty());
            assert!(!group_note(Lang::Id, g).is_empty());
        }
    }

    #[test]
    fn every_tool_card_is_translated() {
        let en = tools(Lang::En);
        let id = tools(Lang::Id);
        let mut differing = 0;
        for (a, b) in en.iter().zip(id.iter()) {
            assert_eq!(a.kind, b.kind);
            assert!(!b.title.is_empty());
            assert!(!b.blurb.is_empty());
            if a.title != b.title {
                differing += 1;
            }
        }
        // "Recycle Bin" is the same word in both, so not every title differs.
        assert!(differing >= 4, "only {differing} titles were translated");
    }
}
