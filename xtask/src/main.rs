use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::process::{Command, Stdio};
use std::time::Instant;

#[derive(Parser)]
#[command(name = "x")]
#[command(about = "Development automation for psrx")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all CI checks (fmt, clippy, build, test)
    Ci {
        #[arg(long)]
        verbose: bool,
    },
    /// Quick checks before commit (fmt, clippy)
    Check {
        #[arg(long)]
        verbose: bool,
    },
    /// Format code
    Fmt {
        #[arg(long)]
        check: bool,
    },
    /// Run clippy
    Clippy {
        #[arg(long)]
        fix: bool,
    },
    /// Build the project
    Build {
        #[arg(long)]
        release: bool,
    },
    /// Run tests
    Test {
        #[arg(long)]
        doc: bool,
        #[arg(long)]
        ignored: bool,
        /// Run only CD-ROM module tests
        #[arg(long)]
        cdrom: bool,
        /// Run only Controller module tests
        #[arg(long)]
        controller: bool,
        /// Run only CPU module tests
        #[arg(long)]
        cpu: bool,
        /// Run only DMA module tests
        #[arg(long)]
        dma: bool,
        /// Run only GPU module tests
        #[arg(long)]
        gpu: bool,
        /// Run only GTE module tests
        #[arg(long)]
        gte: bool,
        /// Run only Interrupt module tests
        #[arg(long)]
        interrupt: bool,
        /// Run only Memory module tests
        #[arg(long)]
        memory: bool,
        /// Run only SPU module tests
        #[arg(long)]
        spu: bool,
        /// Run only System module tests
        #[arg(long)]
        system: bool,
        /// Run only Timer module tests
        #[arg(long)]
        timer: bool,
    },
    /// Run benchmarks
    Bench,
    /// Run BIOS boot test
    BiosBoot {
        /// Path to BIOS file (defaults to SCPH1001.BIN)
        #[arg(default_value = "SCPH1001.BIN")]
        bios_path: String,
        /// Number of instructions to execute (defaults to 100000)
        #[arg(short = 'n', long, default_value = "100000")]
        instructions: u64,
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Pre-commit hook (fmt, clippy, test)
    PreCommit,
    /// Install git hooks
    InstallHooks,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ci { verbose } => run_ci(verbose),
        Commands::Check { verbose } => run_check(verbose),
        Commands::Fmt { check } => run_fmt(check),
        Commands::Clippy { fix } => run_clippy(fix),
        Commands::Build { release } => run_build(release),
        Commands::Test {
            doc,
            ignored,
            cdrom,
            controller,
            cpu,
            dma,
            gpu,
            gte,
            interrupt,
            memory,
            spu,
            system,
            timer,
        } => run_test(
            doc, ignored, cdrom, controller, cpu, dma, gpu, gte, interrupt, memory, spu, system,
            timer,
        ),
        Commands::Bench => run_bench(),
        Commands::BiosBoot {
            bios_path,
            instructions,
            release,
        } => run_bios_boot(&bios_path, instructions, release),
        Commands::PreCommit => run_pre_commit(),
        Commands::InstallHooks => install_hooks(),
    }
}

fn run_ci(verbose: bool) -> Result<()> {
    println!("{}", "=== Running CI Pipeline ===".bold().blue());

    let start = Instant::now();

    run_task("Format Check", || run_fmt(true), verbose)?;
    run_task("Clippy", || run_clippy_ci(), verbose)?;
    run_task("Build", || run_build_ci(), verbose)?;
    run_task(
        "Test",
        || {
            run_test_ci(
                false, false, false, false, false, false, false, false, false, false, false, false,
                false,
            )
        },
        verbose,
    )?;

    let elapsed = start.elapsed();
    println!(
        "\n{} {}",
        "✓ CI passed in".green().bold(),
        format!("{:.2}s", elapsed.as_secs_f64()).bold()
    );

    Ok(())
}

fn run_check(verbose: bool) -> Result<()> {
    println!("{}", "=== Running Quick Checks ===".bold().blue());

    let start = Instant::now();

    run_task("Format Check", || run_fmt(true), verbose)?;
    run_task("Clippy", || run_clippy(false), verbose)?;

    let elapsed = start.elapsed();
    println!(
        "\n{} {}",
        "✓ Checks passed in".green().bold(),
        format!("{:.2}s", elapsed.as_secs_f64()).bold()
    );

    Ok(())
}

fn run_fmt(check: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("fmt").arg("--all");

    if check {
        cmd.arg("--").arg("--check");
    }

    execute_command(&mut cmd)
}

fn run_clippy(fix: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("clippy").arg("--all-targets").arg("--all-features");

    if fix {
        cmd.arg("--fix");
    } else {
        cmd.arg("--").arg("-D").arg("warnings");
    }

    execute_command(&mut cmd)
}

fn run_clippy_ci() -> Result<()> {
    // CI environment: disable default features (audio) to avoid ALSA dependency
    let mut cmd = Command::new("cargo");
    cmd.arg("clippy")
        .arg("--all-targets")
        .arg("--no-default-features")
        .arg("--")
        .arg("-D")
        .arg("warnings");

    execute_command(&mut cmd)
}

fn run_build(release: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build");

    if release {
        cmd.arg("--release");
    }

    execute_command(&mut cmd)
}

fn run_build_ci() -> Result<()> {
    // CI environment: disable default features (audio) to avoid ALSA dependency
    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--no-default-features");

    execute_command(&mut cmd)
}

fn run_test(
    doc: bool,
    ignored: bool,
    cdrom: bool,
    controller: bool,
    cpu: bool,
    dma: bool,
    gpu: bool,
    gte: bool,
    interrupt: bool,
    memory: bool,
    spu: bool,
    system: bool,
    timer: bool,
) -> Result<()> {
    if doc {
        // Run doc tests
        let mut cmd = Command::new("cargo");
        cmd.arg("test").arg("--all-features").arg("--doc");

        if ignored {
            cmd.arg("--").arg("--ignored");
        }

        return execute_command(&mut cmd);
    }

    // Determine which module tests to run
    let module_flags = [
        cdrom, controller, cpu, dma, gpu, gte, interrupt, memory, spu, system, timer,
    ];
    let module_count = module_flags.iter().filter(|&&f| f).count();

    if module_count == 0 {
        // Run all tests
        let mut cmd = Command::new("cargo");
        cmd.arg("test").arg("--all-features");

        if ignored {
            cmd.arg("--").arg("--ignored");
        }

        return execute_command(&mut cmd);
    }

    // Run each module's tests sequentially
    let modules = [
        (cdrom, "core::cdrom", "CD-ROM"),
        (controller, "core::controller", "Controller"),
        (cpu, "core::cpu", "CPU"),
        (dma, "core::dma", "DMA"),
        (gpu, "core::gpu", "GPU"),
        (gte, "core::gte", "GTE"),
        (interrupt, "core::interrupt", "Interrupt"),
        (memory, "core::memory", "Memory"),
        (spu, "core::spu", "SPU"),
        (system, "core::system", "System"),
        (timer, "core::timer", "Timer"),
    ];

    let mut all_success = true;

    for (enabled, module_path, module_name) in modules {
        if !enabled {
            continue;
        }

        println!("{} Running {} tests...", "→".blue(), module_name.bold());

        let mut cmd = Command::new("cargo");
        cmd.arg("test")
            .arg("--all-features")
            .arg("--lib")
            .arg(module_path);

        if ignored {
            cmd.arg("--").arg("--ignored");
        }

        match execute_command(&mut cmd) {
            Ok(_) => {
                println!("{} {} tests passed\n", "✓".green(), module_name);
            }
            Err(e) => {
                println!("{} {} tests failed\n", "✗".red(), module_name);
                all_success = false;
                if module_count == 1 {
                    // If only one module was requested, return the error immediately
                    return Err(e);
                }
            }
        }
    }

    if all_success {
        Ok(())
    } else {
        anyhow::bail!("Some module tests failed")
    }
}

fn run_test_ci(
    doc: bool,
    ignored: bool,
    cdrom: bool,
    controller: bool,
    cpu: bool,
    dma: bool,
    gpu: bool,
    gte: bool,
    interrupt: bool,
    memory: bool,
    spu: bool,
    system: bool,
    timer: bool,
) -> Result<()> {
    // CI environment: disable default features (audio) to avoid ALSA dependency
    if doc {
        // Run doc tests
        let mut cmd = Command::new("cargo");
        cmd.arg("test")
            .arg("--no-default-features")
            .arg("--doc");

        if ignored {
            cmd.arg("--").arg("--ignored");
        }

        return execute_command(&mut cmd);
    }

    // Determine which module tests to run
    let module_flags = [
        cdrom, controller, cpu, dma, gpu, gte, interrupt, memory, spu, system, timer,
    ];
    let module_count = module_flags.iter().filter(|&&f| f).count();

    if module_count == 0 {
        // Run all tests
        let mut cmd = Command::new("cargo");
        cmd.arg("test").arg("--no-default-features");

        if ignored {
            cmd.arg("--").arg("--ignored");
        }

        return execute_command(&mut cmd);
    }

    // Run each module's tests sequentially
    let modules = [
        (cdrom, "core::cdrom", "CD-ROM"),
        (controller, "core::controller", "Controller"),
        (cpu, "core::cpu", "CPU"),
        (dma, "core::dma", "DMA"),
        (gpu, "core::gpu", "GPU"),
        (gte, "core::gte", "GTE"),
        (interrupt, "core::interrupt", "Interrupt"),
        (memory, "core::memory", "Memory"),
        (spu, "core::spu", "SPU"),
        (system, "core::system", "System"),
        (timer, "core::timer", "Timer"),
    ];

    let mut all_success = true;

    for (enabled, module_path, module_name) in modules {
        if !enabled {
            continue;
        }

        println!("{} Running {} tests...", "→".blue(), module_name.bold());

        let mut cmd = Command::new("cargo");
        cmd.arg("test")
            .arg("--no-default-features")
            .arg("--lib")
            .arg(module_path);

        if ignored {
            cmd.arg("--").arg("--ignored");
        }

        match execute_command(&mut cmd) {
            Ok(_) => {
                println!("{} {} tests passed\n", "✓".green(), module_name);
            }
            Err(e) => {
                println!("{} {} tests failed\n", "✗".red(), module_name);
                all_success = false;
                if module_count == 1 {
                    // If only one module was requested, return the error immediately
                    return Err(e);
                }
            }
        }
    }

    if all_success {
        Ok(())
    } else {
        anyhow::bail!("Some module tests failed")
    }
}

fn run_bench() -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("bench");

    execute_command(&mut cmd)
}

fn run_bios_boot(bios_path: &str, instructions: u64, release: bool) -> Result<()> {
    use std::fs;
    use std::path::Path;

    println!("{}", "=== BIOS Boot Test ===".bold().blue());

    // Check if BIOS file exists
    let bios_path_obj = Path::new(bios_path);
    if !bios_path_obj.exists() {
        println!(
            "{} BIOS file not found: {}",
            "✗".red().bold(),
            bios_path.yellow()
        );
        println!(
            "\n{} Please place a valid BIOS file (e.g., SCPH1001.BIN) in the project root.",
            "ℹ".blue()
        );
        anyhow::bail!("BIOS file not found");
    }

    // Verify BIOS file size (should be 512KB)
    let metadata = fs::metadata(bios_path_obj)?;
    if metadata.len() != 512 * 1024 {
        println!(
            "{} Invalid BIOS size: {} bytes (expected 524288 bytes)",
            "✗".red().bold(),
            metadata.len()
        );
        anyhow::bail!("Invalid BIOS file size");
    }

    println!("{} BIOS file: {}", "✓".green(), bios_path.cyan());
    println!(
        "{} Instructions: {}",
        "→".blue(),
        instructions.to_string().bold()
    );
    println!(
        "{} Build mode: {}",
        "→".blue(),
        if release {
            "release".green().bold()
        } else {
            "debug".yellow().bold()
        }
    );
    println!();

    // Build first if needed
    if release {
        println!("{} Building in release mode...", "→".blue());
        run_build(true)?;
        println!();
    }

    // Run the emulator
    let start = Instant::now();

    let mut cmd = Command::new("cargo");
    cmd.arg("run");

    if release {
        cmd.arg("--release");
    }

    cmd.arg("--")
        .arg(bios_path)
        .arg("-n")
        .arg(instructions.to_string());

    let status = cmd
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        println!("\n{} BIOS boot test failed", "✗".red().bold());
        anyhow::bail!("BIOS boot test failed with exit code: {}", status);
    }

    let elapsed = start.elapsed();
    println!(
        "\n{} BIOS boot test completed in {}",
        "✓".green().bold(),
        format!("{:.2}s", elapsed.as_secs_f64()).bold()
    );

    Ok(())
}

fn run_pre_commit() -> Result<()> {
    println!("{}", "=== Pre-commit Checks ===".bold().blue());

    let start = Instant::now();

    run_task("Format Check", || run_fmt(true), false)?;
    run_task("Clippy", || run_clippy(false), false)?;
    run_task(
        "Test",
        || {
            run_test(
                false, false, false, false, false, false, false, false, false, false, false, false,
                false,
            )
        },
        false,
    )?;

    let elapsed = start.elapsed();
    println!(
        "\n{} {}",
        "✓ Pre-commit checks passed in".green().bold(),
        format!("{:.2}s", elapsed.as_secs_f64()).bold()
    );

    Ok(())
}

fn install_hooks() -> Result<()> {
    use std::fs;

    println!("{}", "Installing git hooks...".bold());

    let hook_content = r#"#!/bin/sh
# Auto-generated by cargo x install-hooks
set -e

echo "Running pre-commit checks..."
cargo x pre-commit
"#;

    let hook_path = ".git/hooks/pre-commit";
    fs::write(hook_path, hook_content)?;

    // Make executable (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(hook_path, perms)?;
    }

    println!("{}", "✓ Git hooks installed".green());
    println!("  Pre-commit hook will run: fmt, clippy, test");

    Ok(())
}

fn run_task<F>(name: &str, task: F, verbose: bool) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    print!("{} {} ... ", "→".blue(), name);

    let start = Instant::now();

    match task() {
        Ok(_) => {
            let elapsed = start.elapsed();
            println!(
                "{} {}",
                "✓".green().bold(),
                if verbose {
                    format!("({:.2}s)", elapsed.as_secs_f64())
                } else {
                    String::new()
                }
            );
            Ok(())
        }
        Err(e) => {
            println!("{}", "✗".red().bold());
            Err(e)
        }
    }
}

fn execute_command(cmd: &mut Command) -> Result<()> {
    let status = cmd
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        anyhow::bail!("Command failed with exit code: {}", status);
    }

    Ok(())
}
