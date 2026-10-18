fn main() {
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
    if let Ok(output) = std::process::Command::new("pkg-config")
        .args(["--libs-only-L", "zlib"])
        .output()
    {
        if output.status.success() {
            for flag in String::from_utf8_lossy(&output.stdout).split_whitespace() {
                if let Some(path) = flag.strip_prefix("-L") {
                    println!("cargo:rustc-link-search=native={path}");
                }
            }
        }
    }
    tauri_build::build()
}
