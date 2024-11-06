use ansi_term::Color;
use atty::Stream;
use clap::Parser as _;
use crlf::*;
use std::{fs::File, path::PathBuf};


#[derive(clap::Parser)]
#[command(
    author = "paddydeng@ami.com",
    about = "Check and change line ending for text files",
    version = git_version::git_version!()
)]
struct Cli{
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

fn error_exit<T>(msg: String) -> T {
    eprintln!("{}", msg);
    std::process::exit(1);
}

#[derive(Debug, PartialEq, Eq, clap::ValueEnum, Clone)]
enum Action {
    Measure,
    SetCrlf,
    SetLf,
}

fn main() {
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
                if args.pattern == "**/*" { "*" } else { args.pattern.as_str() },
            ])
            .output()
            .expect("Run git command failed");
        if !git_result.status.success() {
            error_exit(format!(
                "Git command failed: {}",
                git_result
                    .status
                    .code()
                    .expect("Get git status code failed")
            ))
        }
        String::from_utf8_lossy(&git_result.stdout)
            .split('\n')
            .map(|s| s.trim())
            .filter(|s| s.len() > 0)
            .map(|l| PathBuf::from(l))
            .collect()
    } else {
        glob::glob(args.pattern.as_str())
            .expect("Failed to read glob pattern")
            .map(|e| match e {
                Ok(p) => p,
                Err(e) => error_exit(format!("Glob match error {:?}", e)),
            })
            .filter(|f| f.is_file())
            .collect()
    };

    if args.action == Action::Measure {
        files.iter().for_each(|f| {
            let stat = CrlfStat::measure_file(File::open(f).unwrap());
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
        });
    } else {
        let target = match args.action {
            Action::SetCrlf => LineEnding::CRLF,
            Action::SetLf => LineEnding::LF,
            _ => panic!("wtf"),
        };

        files.iter().for_each(|f| {
            let mut dest = vec![];
            convert_to(File::open(f).unwrap(), &mut dest, target);
            std::fs::write(f, dest).unwrap();

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
        });
    }
}
