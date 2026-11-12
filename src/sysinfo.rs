//! Machine details for the System Information panel and the Tools shortcuts.

pub struct SysInfo {
    pub os: String,
    pub cpu: String,
    pub ram_total: u64,
    pub ram_used: u64,
    /// Not simply `total - used`: Windows can still hand back cached memory.
    pub ram_free: u64,
    pub disk_free: u64,
    pub disk_total: u64,
    pub host: String,
    pub user: String,
}

fn windows_version() -> String {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;
        if let Ok(k) = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
        {
            let product: String = k.get_value("ProductName").unwrap_or_default();
            let build: String = k.get_value("CurrentBuild").unwrap_or_default();
            let display: String = k
                .get_value("DisplayVersion")
                .or_else(|_| k.get_value("ReleaseId"))
                .unwrap_or_default();
            if !product.is_empty() {
                return format!("{} (build {}, {})", product, build, display);
            }
        }
    }
    std::env::consts::OS.to_string()
}

pub fn collect() -> SysInfo {
    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();

    let cpu = {
        let brand = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_default();
        if brand.is_empty() {
            "Processor".to_string()
        } else {
            brand
        }
    };

    let host = System::host_name().unwrap_or_else(|| "This PC".into());
    let user = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "me".into());

    // Free space on the drive the account lives on.
    #[cfg(windows)]
    let root = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into()) + "\\";
    #[cfg(not(windows))]
    let root = "/".to_string();
    let (disk_total, disk_free) = disk_space(&root);

    let mem_total = sys.total_memory();
    let mem_used = sys.used_memory();
    // Free memory covers the whole "free" column: what sysinfo leaves unused plus
    // whatever Windows can hand back from its own caches. If those numbers look
    // inconsistent, fall back to plain "total minus used" rather than print nonsense.
    let mem_free = {
        let idle = sys.free_memory();
        let available = idle.saturating_add(sys.available_memory().saturating_sub(idle));
        let available = available.min(mem_total);
        if mem_used <= mem_total && available > 0 && mem_used + available >= mem_total / 2 {
            mem_total.saturating_sub(mem_used).min(available)
        } else {
            mem_total.saturating_sub(mem_used)
        }
    };

    SysInfo {
        os: windows_version(),
        cpu,
        ram_total: mem_total,
        ram_used: mem_used,
        ram_free: mem_free,
        disk_free,
        disk_total,
        host,
        user,
    }
}

#[cfg(windows)]
fn disk_space(root: &str) -> (u64, u64) {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(
            dir: *const u16,
            caller: *mut u64,
            total: *mut u64,
            free: *mut u64,
        ) -> i32;
    }
    let mut wide: Vec<u16> = std::ffi::OsStr::new(root)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let (mut free_to_caller, mut total, mut free) = (0u64, 0u64, 0u64);
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_mut_ptr(),
            &mut free_to_caller,
            &mut total,
            &mut free,
        )
    };
    if ok != 0 {
        (total, free)
    } else {
        (0, 0)
    }
}

#[cfg(not(windows))]
fn disk_space(_root: &str) -> (u64, u64) {
    (0, 0)
}

/// Places the Tools tab can open. Kept separate so the UI stays dumb.
pub fn tool_target(kind: &str) -> Option<String> {
    let win = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let user = std::env::var("USERPROFILE").unwrap_or_default();
    let p = match kind {
        "uninstall" => format!("{}\\System32\\appwiz.cpl", win),
        "startup" => "shell:startup".to_string(),
        "services" => format!("{}\\System32\\services.msc", win),
        "recycle" => "shell:RecycleBinFolder".to_string(),
        "temp" => {
            std::env::var("TEMP").unwrap_or_else(|_| format!("{}\\AppData\\Local\\Temp", user))
        }
        "downloads" => format!("{}\\Downloads", user),
        "addons" => format!("{}\\System32\\control.exe", win),
        _ => return None,
    };
    Some(p)
}
