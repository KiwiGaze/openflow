fn main() {
    // ggml's Metal backend uses Objective-C `@available()` checks, which lower
    // to `___isPlatformVersionAtLeast` from clang's compiler-rt. rustc links
    // with `-nodefaultlibs`, so that builtin library must be added manually or
    // release (LTO) links fail with an undefined-symbol error.
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("clang")
            .arg("--print-resource-dir")
            .output()
        {
            let resource_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-link-search={resource_dir}/lib/darwin");
            println!("cargo:rustc-link-lib=static=clang_rt.osx");
        }
    }

    tauri_build::build()
}
