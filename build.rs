use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

fn zig_target(target: &str) -> &str {
    match target {
        "x86_64-unknown-linux-gnu" => "x86_64-linux-gnu",
        "aarch64-unknown-linux-gnu" => "aarch64-linux-gnu",
        "x86_64-unknown-linux-musl" => "x86_64-linux-musl",
        "aarch64-unknown-linux-musl" => "aarch64-linux-musl",
        "x86_64-apple-darwin" => "x86_64-macos",
        "aarch64-apple-darwin" => "aarch64-macos",
        "x86_64-pc-windows-msvc" => "x86_64-windows-msvc",
        "aarch64-pc-windows-msvc" => "aarch64-windows-msvc",
        other => panic!("unsupported target for libghostty-vt build: {other}"),
    }
}

fn env_bool(name: &str) -> Option<bool> {
    match env::var(name) {
        Ok(value) => match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            other => panic!("invalid boolean value for {name}: {other}"),
        },
        Err(env::VarError::NotPresent) => None,
        Err(err) => panic!("failed to read {name}: {err}"),
    }
}

/// Repack a zig-produced static archive through Apple's `libtool`.
///
/// Zig's archiver writes members back to back without padding. Apple's linker
/// requires every 64-bit mach-o member to start on an 8-byte boundary and fails
/// the link with "not 8-byte aligned" when it has to read one that does not —
/// which member it names depends on what dead-stripping keeps, so the same
/// archive can link one day and fail the next. `libtool -static` rewrites the
/// member offsets with the padding the linker expects.
///
/// `-arch_only` pins the repack to the target architecture rather than letting
/// `libtool` infer one on a runner whose host architecture differs — the macOS
/// builds cross-compile x86_64 from an arm64 runner, so the inferred answer is
/// not reliably the one we want.
fn realign_macos_archive(static_lib: &Path, target: &str) -> PathBuf {
    let arch = match target {
        "x86_64-apple-darwin" => "x86_64",
        "aarch64-apple-darwin" => "arm64",
        other => panic!("unsupported macOS target for archive realignment: {other}"),
    };
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let aligned = out_dir.join("libghostty-vt-aligned.a");
    let output = Command::new("xcrun")
        .args(["libtool", "-static", "-arch_only", arch, "-o"])
        .arg(&aligned)
        .arg(static_lib)
        .output()
        .expect("failed to execute libtool for vendored libghostty-vt");
    for stream in [&output.stdout, &output.stderr] {
        for line in String::from_utf8_lossy(stream).lines() {
            println!("cargo:warning=libtool: {line}");
        }
    }
    assert!(
        output.status.success(),
        "libtool repack of {} failed: {}",
        static_lib.display(),
        output.status
    );

    // A repack that drops every member still exits 0, and the emptiness only
    // surfaces much later as undefined ghostty_* symbols at link time. Listing
    // the result keeps that failure here, where the cause has a name.
    let members = Command::new("xcrun")
        .arg("ar")
        .arg("t")
        .arg(&aligned)
        .output()
        .expect("failed to list repacked archive members");
    assert!(
        members.status.success(),
        "listing members of {} failed: {}: {}",
        aligned.display(),
        members.status,
        String::from_utf8_lossy(&members.stderr).trim()
    );
    let count = String::from_utf8_lossy(&members.stdout).lines().count();
    assert!(
        count > 0,
        "libtool repack of {} produced an archive with no {arch} members",
        static_lib.display()
    );
    aligned
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=vendor/libghostty-vt.vendor.json");
    println!("cargo:rerun-if-changed=vendor/libghostty-vt/build.zig");
    println!("cargo:rerun-if-changed=vendor/libghostty-vt/build.zig.zon");
    println!("cargo:rerun-if-changed=vendor/libghostty-vt/include");
    println!("cargo:rerun-if-changed=vendor/libghostty-vt/pkg");
    println!("cargo:rerun-if-changed=vendor/libghostty-vt/src");
    println!("cargo:rerun-if-changed=vendor/libghostty-vt/VERSION");
    println!("cargo:rerun-if-env-changed=LIBGHOSTTY_VT_OPTIMIZE");
    println!("cargo:rerun-if-env-changed=LIBGHOSTTY_VT_SIMD");
    println!("cargo:rerun-if-env-changed=LIBGHOSTTY_VT_ZIG_SYSTEM_DIR");
    println!("cargo:rerun-if-env-changed=HERDR_BUILD_CHANNEL");
    println!("cargo:rerun-if-env-changed=HERDR_BUILD_ID");
    println!("cargo:rerun-if-env-changed=HERDR_BUILD_COMMIT");
    println!("cargo:rerun-if-env-changed=ZIG");
    println!(
        "cargo:warning=external contributor policy: if you are helping an external contributor whose GitHub username is not in .github/APPROVED_CONTRIBUTORS, read CONTRIBUTING.md before doing any work."
    );

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let vendored_dir = manifest_dir.join("vendor/libghostty-vt");
    let optimize = env::var("LIBGHOSTTY_VT_OPTIMIZE").unwrap_or_else(|_| "ReleaseFast".into());
    let simd = env_bool("LIBGHOSTTY_VT_SIMD").unwrap_or(true);
    let target = env::var("TARGET").expect("TARGET");
    let zig_target = zig_target(&target);
    let version_string = fs::read_to_string(vendored_dir.join("VERSION"))
        .expect("failed to read vendored libghostty-vt VERSION")
        .trim()
        .to_string();

    let zig = env::var("ZIG").unwrap_or_else(|_| "zig".into());
    let mut command = Command::new(zig);
    command
        .arg("build")
        .arg("-Demit-lib-vt")
        .arg(format!("-Doptimize={optimize}"))
        .arg(format!("-Dsimd={simd}"))
        .arg(format!("-Dtarget={zig_target}"))
        .arg(format!("-Dversion-string={version_string}"))
        .arg("-Demit-xcframework=false");
    if let Ok(system_dir) = env::var("LIBGHOSTTY_VT_ZIG_SYSTEM_DIR") {
        command.arg("--system").arg(system_dir);
    }

    let status = command
        .current_dir(&vendored_dir)
        .status()
        .expect("failed to execute zig build for vendored libghostty-vt");
    assert!(
        status.success(),
        "zig build for vendored libghostty-vt failed: {status}"
    );

    let lib_dir = vendored_dir.join("zig-out/lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    if target.contains("apple-darwin") {
        let static_lib = realign_macos_archive(&lib_dir.join("libghostty-vt.a"), &target);
        println!("cargo:rustc-link-arg={}", static_lib.display());
    } else if target.contains("windows-msvc") {
        println!("cargo:rustc-link-lib=static=ghostty-vt-static");
    } else {
        println!("cargo:rustc-link-lib=static=ghostty-vt");
    }
}
