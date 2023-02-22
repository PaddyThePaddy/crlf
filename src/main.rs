use ansi_term::Color;
use clap::{Arg, ArgAction, ArgMatches};
use crlf::*;
use std::{fs::File, path::PathBuf};

fn build_args() -> ArgMatches {
    clap::Command::new("utf8-test")
    .author("paddydeng@ami.com")
    .version(git_version::git_version!())
    .about("Check if files has illegal character for given encoding")
    .arg(
      Arg::new("action")
      .action(ArgAction::Set).required(true)
    )
    .arg(
      Arg::new("pattern")
      .default_value("**/*")
      .action(ArgAction::Set)
      .help("file name pattern (using glob)\nif --git-file(-g) is given, this pattern will be passed to git ls-files")
    )
    .arg(
      Arg::new("git-file")
      .short('g')
      .long("git-file")
      .action(ArgAction::SetTrue)
      .help("Use git ls-files to get file list")
    )
    .arg(
      Arg::new("verbose")
      .short('v')
      .long("verbose")
      .action(ArgAction::SetTrue)
      .help("Show detailed output")
    )
    .get_matches()
}

fn error_exit<T>(msg: String) -> T {
    eprintln!("{}", msg);
    std::process::exit(1);
}

#[derive(Debug, PartialEq, Eq)]
enum Action {
    Measure,
    SetCrlf,
    SetLf,
}

fn main() {
    let args = build_args();
    let action = match args.get_one::<String>("action").unwrap().as_str() {
        "measure" => Action::Measure,
        "to-crlf" => Action::SetCrlf,
        "to-lf" => Action::SetLf,
        _ => error_exit(format!("must provide an action: measure, to-crlf, to-lf")),
    };
    let pattern = args
        .get_one::<String>("pattern")
        .expect("No pattern provided");

    let files: Vec<PathBuf> = if args.get_flag("git-file") {
        let git_result = std::process::Command::new("git")
            .args([
                "ls-files",
                "--",
                if pattern == "**/*" { "*" } else { pattern },
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
        glob::glob(pattern)
            .expect("Failed to read glob pattern")
            .map(|e| match e {
                Ok(p) => p,
                Err(e) => error_exit(format!("Glob match error {:?}", e)),
            })
            .filter(|f| f.is_file())
            .collect()
    };

    if action == Action::Measure {
        files.iter().for_each(|f| {
            let stat = CrlfStat::measure_file(File::open(f).unwrap());
            let filename = f.as_os_str().to_string_lossy();
            let name_str = match stat.is_pure() {
                Some(le) => match le {
                    LineEnding::CRLF => Color::Yellow.paint(filename),
                    LineEnding::LF => Color::Green.paint(filename),
                },
                None => Color::Red.paint(filename),
            };
            println!(
                "{}: crlf: {}, lf: {}",
                name_str,
                Color::Yellow.paint(format!("{}", stat.crlf())),
                Color::Green.paint(format!("{}", stat.lf()))
            );
        });
    } else {
        let target = match action {
            Action::SetCrlf => LineEnding::CRLF,
            Action::SetLf => LineEnding::LF,
            _ => panic!("wtf"),
        };

        files.iter().for_each(|f| {
            let mut dest = vec![];
            convert_to(File::open(f).unwrap(), &mut dest, target);
            std::fs::write(f, dest).unwrap();

            println!(
                "set {} to {}",
                f.display(),
                match target {
                    LineEnding::CRLF => Color::Yellow.paint(format!("{}", target)),
                    LineEnding::LF => Color::Green.paint(format!("{}", target)),
                }
            );
        });
    }
}
