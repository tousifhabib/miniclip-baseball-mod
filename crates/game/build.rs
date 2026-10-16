//! Puts the app's icon, and the name Windows shows for it, inside the Windows
//! program, when the build is given an icon to put there.
//! `scripts/bundle-windows.ps1` draws one from `assets/icon.svg` and names
//! it in `BB_WINDOWS_ICON`. Any other build has nothing to do here.

fn main() {
    println!("cargo:rerun-if-env-changed=BB_WINDOWS_ICON");
    #[cfg(windows)]
    if let Some(icon) = std::env::var_os("BB_WINDOWS_ICON") {
        windows::put_in(&icon);
    }
}

#[cfg(windows)]
mod windows {
    use std::ffi::OsStr;

    /// What the app is called, as `scripts/bundle-windows.ps1` names it.
    const APP_NAME: &str = "Baseball Mod";

    /// Builds the icon and the program's account of itself into the
    /// program. An icon that was asked for and cannot be put in stops the
    /// build, so that no app goes out without it.
    pub(super) fn put_in(icon: &OsStr) {
        let icon = icon.to_str().expect("the icon's path is not text");
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon(icon)
            .set("ProductName", APP_NAME)
            .set("FileDescription", APP_NAME)
            .set("OriginalFilename", &format!("{APP_NAME}.exe"));
        if let Err(error) = resource.compile() {
            panic!("putting {icon} in the program: {error}");
        }
    }
}
