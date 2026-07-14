use dialoguer::{Input, Select, theme::ColorfulTheme};
use std::process::Command;
use sysinfo::System;
use xshell::{Shell, cmd};

/// Returns true if an external command binary is resolvable on PATH.
fn binary_present(bin: &str) -> bool {
    #[cfg(windows)]
    let (probe, arg) = ("where", bin);
    #[cfg(not(windows))]
    let (probe, arg) = ("which", bin);
    Command::new(probe)
        .arg(arg)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let sh = Shell::new()?;

    if args.len() < 2 {
        return cockpit(&sh);
    }

    match args[1].as_str() {
        "quick" => {
            quick(&sh)?;
        }
        "fullscale" => {
            fullscale(&sh)?;
        }
        _ => {
            println!("Usage: cargo xtask [quick|fullscale]");
        }
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn cockpit(sh: &Shell) -> Result<(), Box<dyn std::error::Error>> {
    let selections = &[
        "[1] INNER LOOP: Quick Test (Build & Launch)",
        "[2] QUALITY GATE: Format & Lint Workspace",
        "[3] QUALITY GATE: Run Unit Tests",
        "[4] QUALITY GATE: Coverage Report (llvm-cov)",
        "[5] QUALITY GATE: Supply-chain & Hygiene (advisory)",
        "[6] SHIP & RELEASE: Fullscale Workflow (Commit & Push)",
        "[7] SHIP & RELEASE: Version Bump (lockstep)",
        "[8] SHIP & RELEASE: Build Windows Installer (local)",
        "[0] Quit",
    ];

    loop {
        println!("\n========================================");
        println!("🚀 Rust Developers Cockpit 🚀");
        println!("========================================");

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select an action")
            .default(0)
            .items(&selections[..])
            .interact()?;

        match selection {
            0 => {
                if let Err(e) = quick(sh) {
                    println!("Error: {e}");
                }
            }
            1 => {
                println!("=== Formatting ===");
                cmd!(sh, "cargo fmt").run().ok();
                println!("=== Linting ===");
                cmd!(
                    sh,
                    "cargo clippy --workspace --all-targets --all-features -- -D warnings"
                )
                .run()
                .ok();
            }
            2 => {
                println!("=== Running Tests ===");
                if binary_present("cargo-nextest") {
                    cmd!(sh, "cargo nextest run").run().ok();
                } else {
                    println!(
                        "cargo-nextest not found — falling back to `cargo test`. \
                         Install it for faster runs: cargo install cargo-nextest --locked"
                    );
                    cmd!(sh, "cargo test").run().ok();
                }
            }
            3 => {
                println!("=== Coverage (llvm-cov + nextest) ===");
                if binary_present("cargo-llvm-cov") {
                    cmd!(sh, "cargo llvm-cov nextest").run().ok();
                } else {
                    println!(
                        "cargo-llvm-cov not found. Install it to generate coverage: \
                         cargo install cargo-llvm-cov --locked \
                         (also needs the llvm-tools-preview rustup component)"
                    );
                }
            }
            4 => {
                println!("=== Supply-chain & Hygiene (advisory — CI is the gate) ===");
                if binary_present("cargo-deny") {
                    cmd!(sh, "cargo deny check").run().ok();
                } else {
                    println!("cargo-deny not found: cargo install cargo-deny --locked");
                }
                if binary_present("cargo-machete") {
                    cmd!(sh, "cargo machete").run().ok();
                } else {
                    println!("cargo-machete not found: cargo install cargo-machete --locked");
                }
                if binary_present("typos") {
                    cmd!(sh, "typos").run().ok();
                } else {
                    println!("typos not found: cargo install typos-cli --locked");
                }
            }
            5 => {
                if let Err(e) = fullscale(sh) {
                    println!("Error: {e}");
                }
            }
            6 => {
                if let Err(e) = version_bump(sh) {
                    println!("Error: {e}");
                }
            }
            7 => build_windows_installer(sh)?,
            8 => {
                println!("Exiting Cockpit...");
                break;
            }
            _ => unreachable!(),
        }

        let _: String = Input::new()
            .with_prompt("Press Enter to continue...")
            .allow_empty(true)
            .interact_text()?;
    }

    Ok(())
}

fn kill_processes() {
    let mut sys = System::new_all();
    sys.refresh_all();

    // Kill processes named "backend.exe" or "frontend.exe"
    for process in sys.processes().values() {
        let name = process.name().to_string_lossy().to_lowercase();
        if name == "backend.exe"
            || name == "frontend.exe"
            || name == "backend"
            || name == "frontend"
        {
            println!(
                "Killing existing process: {} (PID: {})",
                name,
                process.pid()
            );
            process.kill();
        }
    }
}

fn quick(sh: &Shell) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Running Lefthook pre-commit checks ===");
    #[cfg(windows)]
    cmd!(sh, "cmd.exe /c lefthook run pre-commit").run()?;
    #[cfg(not(windows))]
    cmd!(sh, "lefthook run pre-commit").run()?;

    println!("=== Killing running processes before build ===");
    kill_processes();

    // Small delay to ensure OS file locks are released
    std::thread::sleep(std::time::Duration::from_millis(500));

    println!("=== Building workspace ===");
    cmd!(sh, "cargo build --workspace --exclude xtask").run()?;

    println!("=== Launching processes ===");
    let backend_log = std::fs::File::create("backend.log")?;
    let frontend_log = std::fs::File::create("frontend.log")?;

    #[cfg(target_os = "windows")]
    {
        Command::new("target\\debug\\backend.exe")
            .stdout(backend_log.try_clone()?)
            .stderr(backend_log)
            .spawn()?;

        Command::new("target\\debug\\frontend.exe")
            .stdout(frontend_log.try_clone()?)
            .stderr(frontend_log)
            .spawn()?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("./target/debug/backend")
            .stdout(backend_log.try_clone()?)
            .stderr(backend_log)
            .spawn()?;

        Command::new("./target/debug/frontend")
            .stdout(frontend_log.try_clone()?)
            .stderr(frontend_log)
            .spawn()?;
    }

    println!("=== Quick test workflow complete! ===");
    Ok(())
}

fn fullscale(sh: &Shell) -> Result<(), Box<dyn std::error::Error>> {
    // Run the quick workflow first
    quick(sh)?;

    println!("=== Staging changes ===");
    cmd!(sh, "git add .").run()?;

    let message: String = if let Some(msg) = std::env::args().nth(2) {
        msg
    } else {
        Input::new()
            .with_prompt("Enter commit message")
            .interact_text()?
    };

    println!("=== Committing ===");
    cmd!(sh, "git commit -m {message} --no-verify").run()?;

    println!("=== Running pre-push hooks ===");
    #[cfg(windows)]
    cmd!(sh, "cmd.exe /c lefthook run pre-push").run()?;
    #[cfg(not(windows))]
    cmd!(sh, "lefthook run pre-push").run()?;

    println!("=== Pushing ===");
    cmd!(sh, "git push --no-verify").run()?;

    println!("=== Watching CI ===");
    cmd!(sh, "gh run watch").run()?;

    println!("=== Fullscale workflow complete! ===");
    Ok(())
}

fn version_bump(sh: &Shell) -> Result<(), Box<dyn std::error::Error>> {
    let frontend_cargo = sh.read_file("frontend/Cargo.toml")?;
    let mut current_version = String::new();
    for line in frontend_cargo.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version = \"") {
            current_version = trimmed
                .trim_start_matches("version = \"")
                .trim_end_matches('"')
                .to_string();
            break;
        }
    }

    if current_version.is_empty() {
        println!("Could not detect current version.");
        return Ok(());
    }

    println!("Current version: {current_version}");

    let selections = &["Patch", "Minor", "Major", "Cancel"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select bump level")
        .default(0)
        .items(&selections[..])
        .interact()?;

    let mut parts: Vec<u32> = current_version
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    if parts.len() != 3 {
        println!("Invalid semver format: {current_version}");
        return Ok(());
    }

    match selection {
        0 => parts[2] += 1, // Patch
        1 => {
            parts[1] += 1;
            parts[2] = 0;
        } // Minor
        2 => {
            parts[0] += 1;
            parts[1] = 0;
            parts[2] = 0;
        } // Major
        _ => {
            println!("Version bump cancelled.");
            return Ok(());
        }
    }

    let new_version = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
    println!("Bumping from {current_version} to {new_version}...");

    let tomls = vec![
        "Cargo.toml",
        "frontend/Cargo.toml",
        "backend/Cargo.toml",
        "common/Cargo.toml",
        "xtask/Cargo.toml",
    ];

    let old_line = format!("version = \"{current_version}\"");
    let new_line = format!("version = \"{new_version}\"");

    for toml in tomls {
        if let Ok(content) = sh.read_file(toml)
            && content.contains(&old_line)
        {
            let new_content = content.replacen(&old_line, &new_line, 1);
            sh.write_file(toml, new_content)?;
            println!("Updated {toml}");
        }
    }

    println!("Version bump complete! Do not forget to commit your changes.");
    Ok(())
}

/// Builds `rsahp-desktop` (release) and runs Inno Setup's `iscc`, injecting the version
/// from `cargo pkgid` (robust — no JSON string-slicing). Windows-only.
// The non-windows cfg arm is a stub that always returns `Ok(())`, so clippy flags the
// `Result` as unnecessary when checked on macOS/Linux — but the windows arm genuinely
// uses `?`, so the wrap IS needed on the target that actually runs this.
#[allow(clippy::unnecessary_wraps)]
fn build_windows_installer(sh: &Shell) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(windows))]
    {
        let _ = sh;
        println!("Windows installer build is only supported on Windows.");
        Ok(())
    }
    #[cfg(windows)]
    {
        cmd!(sh, "cargo build --release -p rsahp-desktop").run()?;

        // `cargo pkgid -p rsahp-desktop` → e.g. "path+file:///…#rsahp-desktop@0.1.0"
        // (or "…#0.1.0"). Take the version after the last '@', else after the last '#'.
        let pkgid = cmd!(sh, "cargo pkgid -p rsahp-desktop").read()?;
        let pkgid = pkgid.trim();
        let version = pkgid
            .rsplit_once('@')
            .map(|(_, v)| v)
            .or_else(|| pkgid.rsplit_once('#').map(|(_, v)| v))
            .ok_or("could not parse version from cargo pkgid")?
            .to_string();
        println!("Building installer for rsahp-desktop v{version}");

        let def = format!("/DMyAppVersion={version}");
        cmd!(sh, "iscc {def} packaging/windows/rsahp.iss").run()?;
        println!("Installer written to dist/rsahp-setup-{version}.exe");
        Ok(())
    }
}
