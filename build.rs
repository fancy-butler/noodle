// Embeds the app icon in the Windows .exe. Does nothing on other platforms.
fn main() {
    println!("cargo:rerun-if-changed=assets/noodle.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/noodle.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=couldn't embed the icon: {e}");
        }
    }
}
