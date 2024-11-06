use ansi_term::Color;
use anyhow::{anyhow, Context};
use atty::Stream;
use clap::Parser as _;
use crlf::*;
use std::{fs::File, io::BufReader, path::PathBuf};

#[derive(clap::Parser)]
#[command(
    author = "paddydeng@ami.com",
    about = "Check and change line ending for text files",
    version = git_version::git_version!()
)]
struct Cli {
    action: Action,
    /// file name pattern (using glob)
    ///
    /// if --git-file(-g) is given, this pattern will be passed to git ls-files
    #[arg(default_value = "**/*")]
    pattern: String,

    /// Use git grep to get text file list
    #[arg(long, short)]
    git_file: bool,

    /// Show detailed output
    #[arg(long, short)]
    verbose: bool,
}

#[derive(Debug, PartialEq, Eq, clap::ValueEnum, Clone)]
enum Action {
    Measure,
    SetCrlf,
    SetLf,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let files: Vec<PathBuf> = if args.git_file {
        let git_result = std::process::Command::new("git")
            .args([
                "grep",
                "-I",
                "--name-only",
                "--untracked",
                "-e",
                ".",
                "--",
                if args.pattern == "**/*" {
                    "*"
                } else {
                    args.pattern.as_str()
                },
            ])
            .output()
            .context("Run git command failed")?;
        if !git_result.status.success() {
            if let Some(code) = git_result.status.code() {
                return Err(anyhow!("Git command failed with exit code: {code}"));
            } else {
                return Err(anyhow!("git exit unexpectly without an exit code"));
            }
        }
        String::from_utf8_lossy(&git_result.stdout)
            .split('\n')
            .map(|s| s.trim())
            .filter(|s| s.len() > 0)
            .map(|l| PathBuf::from(l))
            .collect()
    } else {
        glob::glob(args.pattern.as_str())
            .context("Failed to read glob pattern")?
            .filter(|f| f.as_ref().is_ok_and(|f| f.is_file()))
            .collect::<Result<Vec<_>, _>>()
            .context("Glob match error")?
    };

    if args.action == Action::Measure {
        files
            .iter()
            .map(|f| {
                let stat = CrlfStat::measure_file(
                    File::open(f).context(format!("Read file {} failed", f.display()))?,
                );
                if atty::is(Stream::Stdout) {
                    let indicator = match stat.is_pure() {
                        Some(le) => match le {
                            LineEnding::CRLF => Color::Yellow.paint("C"),
                            LineEnding::LF => Color::Green.paint("L"),
                        },
                        None => Color::Red.paint("X"),
                    };
                    println!(
                        "{}, {}, {}, {}",
                        indicator,
                        Color::Yellow.paint(format!("crlf: {:4}", stat.crlf())),
                        Color::Green.paint(format!("lf: {:4}", stat.lf())),
                        f.display(),
                    );
                } else {
                    let indicator = match stat.is_pure() {
                        Some(le) => match le {
                            LineEnding::CRLF => 'C',
                            LineEnding::LF => 'L',
                        },
                        None => 'X',
                    };
                    println!(
                        "{}, crlf: {:4}, lf: {:4}, {}",
                        indicator,
                        stat.crlf(),
                        stat.lf(),
                        f.display(),
                    );
                }
                Ok::<(), anyhow::Error>(())
            })
            .collect::<Result<(), anyhow::Error>>()?;
    } else {
        let target = match args.action {
            Action::SetCrlf => LineEnding::CRLF,
            Action::SetLf => LineEnding::LF,
            _ => panic!("wtf"),
        };

        files
            .iter()
            .map(|f| {
                let mut dest = vec![];
                convert_to(
                    BufReader::new(
                        File::open(f).context(format!("Read file {} failed", f.display()))?,
                    ),
                    &mut dest,
                    target,
                )
                .context(format!("Convert file {} failed", f.display()))?;
                std::fs::write(f, dest).context(format!("Write file {} failed", f.display()))?;

                if atty::is(Stream::Stdout) {
                    println!(
                        "set {} to {}",
                        f.display(),
                        match target {
                            LineEnding::CRLF => Color::Yellow.paint(format!("{}", target)),
                            LineEnding::LF => Color::Green.paint(format!("{}", target)),
                        }
                    );
                } else {
                    println!("set {} to {}", f.display(), target);
                }

                Ok::<(), anyhow::Error>(())
            })
            .collect::<Result<(), anyhow::Error>>()?;
    }

    Ok(())
}
