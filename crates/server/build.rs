use std::{io::Result, process::Command};

fn main() -> Result<()> {
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=Rstrtmgr");
    }

    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .expect("Failed to get git commit hash");
    let git_hash = String::from_utf8(output.stdout)
        .expect("Invalid UTF-8")
        .trim()
        .to_string();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    // Build frontend assets
    build_frontend();

    Ok(())
}

fn build_frontend() {
    let web_dir = std::path::Path::new("../../web");
    let dist_dir = web_dir.join("dist");

    // Re-run build.rs when frontend source or dependencies change.
    println!("cargo:rerun-if-changed=../../web/package.json");
    println!("cargo:rerun-if-changed=../../web/pnpm-lock.yaml");
    println!("cargo:rerun-if-changed=../../web/tsconfig.json");
    println!("cargo:rerun-if-changed=../../web/vite.config.ts");
    println!("cargo:rerun-if-changed=../../web/index.html");
    println!("cargo:rerun-if-changed=../../web/src/");

    // Find a working pnpm command. On Windows it may only be available
    // via cmd.exe (PATH set by the installer, not inherited by bash).
    let mut pnpm_cmd = None;
    for candidate in &["pnpm", "pnpm.cmd"] {
        if command_ok(candidate, &["--version"], web_dir) {
            pnpm_cmd = Some(*candidate);
            break;
        }
    }
    // Windows fallback: run through cmd.exe which has the full system PATH.
    if pnpm_cmd.is_none() && cfg!(target_os = "windows") {
        if command_ok("cmd", &["/c", "pnpm.cmd", "--version"], web_dir) {
            pnpm_cmd = Some("cmd");
        }
    }

    let Some(cmd) = pnpm_cmd else {
        // If dist/ already exists (built manually), allow the build to proceed.
        if dist_dir.exists() {
            eprintln!(
                "WARNING: `pnpm` not found in PATH but `web/dist/` exists.\n\
                 The embedded frontend may be stale. Install pnpm and run\n\
                 `cd web && pnpm run build` to update it.\n"
            );
            return;
        }
        eprintln!(
            "\nERROR: `pnpm` not found. Install Node.js and pnpm, then run:\n\
               cd web && pnpm install && pnpm run build\n"
        );
        std::process::exit(1);
    };

    if !web_dir.join("node_modules").exists() {
        // Fresh clone — install dependencies first.
        eprintln!("Installing frontend dependencies (one-time setup)...");
        if !run_pnpm(cmd, &["install"], web_dir) {
            eprintln!("\nERROR: `pnpm install` failed. See output above.\n");
            std::process::exit(1);
        }
    }

    if !run_pnpm(cmd, &["run", "build"], web_dir) {
        eprintln!("\nERROR: `pnpm run build` failed. See output above.\n");
        std::process::exit(1);
    }
}

fn command_ok(cmd: &str, args: &[&str], dir: &std::path::Path) -> bool {
    Command::new(cmd)
        .args(args)
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run_pnpm(cmd: &str, args: &[&str], dir: &std::path::Path) -> bool {
    if cmd == "cmd" {
        let mut full_args = vec!["/c", "pnpm.cmd"];
        full_args.extend_from_slice(args);
        Command::new("cmd")
            .args(&full_args)
            .current_dir(dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        Command::new(cmd)
            .args(args)
            .current_dir(dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}
