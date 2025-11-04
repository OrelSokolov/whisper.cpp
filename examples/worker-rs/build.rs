use std::env;
use std::path::PathBuf;

fn main() {
    // Tell cargo to link against whisper library
    // CARGO_MANIFEST_DIR is examples/worker-rs, so we need to go up 2 levels to get to whisper.cpp root
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = PathBuf::from(&manifest_dir);
    let whisper_root = manifest_path
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    
    let build_dir = whisper_root.join("build");
    
    // Convert to absolute paths for linker
    let src_path = build_dir.join("src");
    let examples_path = build_dir.join("examples");
    let ggml_src_path = build_dir.join("ggml").join("src");
    
    // Add search paths for libraries (use absolute paths)
    // Note: canonicalize only works if path exists, so we use it conditionally
    let src_abs = if src_path.exists() {
        src_path.canonicalize().unwrap_or(src_path)
    } else {
        src_path
    };
    let examples_abs = if examples_path.exists() {
        examples_path.canonicalize().unwrap_or(examples_path)
    } else {
        examples_path
    };
    let ggml_abs = if ggml_src_path.exists() {
        ggml_src_path.canonicalize().unwrap_or(ggml_src_path)
    } else {
        ggml_src_path
    };
    
    println!("cargo:rustc-link-search=native={}", src_abs.display());
    println!("cargo:rustc-link-search=native={}", examples_abs.display());
    println!("cargo:rustc-link-search=native={}", ggml_abs.display());
    
    // Add rpath so the executable can find shared libraries at runtime
    // Skip setting local rpath when building for debian package (RPATH will be set via RUSTFLAGS)
    let skip_local_rpath = env::var("DEB_BUILD_ARCH").is_ok() || env::var("DEBIAN_BUILD").is_ok();
    
    if !skip_local_rpath {
        let src_rpath = if src_abs.exists() {
            src_abs.canonicalize().unwrap_or(src_abs)
        } else {
            src_abs
        };
        let ggml_rpath = if ggml_abs.exists() {
            ggml_abs.canonicalize().unwrap_or(ggml_abs)
        } else {
            ggml_abs
        };
        
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", src_rpath.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", ggml_rpath.display());
    }
    
    // Link libraries (whisper and ggml are shared, common is static)
    println!("cargo:rustc-link-lib=dylib=whisper");
    println!("cargo:rustc-link-lib=static=common");
    println!("cargo:rustc-link-lib=dylib=ggml");
    
    // Link system libraries
    println!("cargo:rustc-link-lib=stdc++");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=ssl");
    println!("cargo:rustc-link-lib=crypto");
    
    // Tell cargo to invalidate the built crate whenever these change
    println!("cargo:rerun-if-changed=build.rs");
}

