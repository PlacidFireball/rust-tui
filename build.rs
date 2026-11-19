use std::process::Command;

/// Embeds the Python interpreter's lib directory as an rpath so the binary can
/// find `libpython3.x.dylib` at runtime (pyo3 links it dynamically).
fn main() {
    let python = std::env::var("PYO3_PYTHON").unwrap_or_else(|_| "python3".to_string());
    let output = Command::new(&python)
        .args(["-c", "import sysconfig; print(sysconfig.get_config_var('LIBDIR'))"])
        .output()
        .expect("failed to run python to locate LIBDIR");
    let lib_dir = String::from_utf8(output.stdout).expect("LIBDIR was not valid UTF-8");

    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.trim());
    println!("cargo:rerun-if-env-changed=PYO3_PYTHON");
}
