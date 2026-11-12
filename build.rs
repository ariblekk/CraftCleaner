fn main() {
    println!("cargo:rerun-if-changed=icon.ico");

    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icon.ico");
        res.set("ProductName", "Cleaner");
        res.set("FileDescription", "A system cleaner");
        res.set("LegalCopyright", "");
        res.compile().expect("failed to compile Windows resources");
    }
}
