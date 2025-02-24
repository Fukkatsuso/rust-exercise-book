use std::fs;
use std::os::unix::fs::MetadataExt;
use std::{error::Error, path::PathBuf};

use chrono::{DateTime, Local};
use clap::{App, Arg};
use tabular::{Row, Table};

// mod owner;
// use owner::Owner;

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    paths: Vec<String>,
    long: bool,
    show_hidden: bool,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("lsr")
        .version("0.1.0")
        .author("Fukkatsuso <fukkatsuso.git+github@gmail.com>")
        .about("Rust ls")
        .arg(
            Arg::with_name("paths")
                .value_name("PATH")
                .help("Files and/or directories")
                .multiple(true)
                .default_value("."),
        )
        .arg(
            Arg::with_name("all")
                .short("a")
                .long("all")
                .takes_value(false)
                .help("Show all files"),
        )
        .arg(
            Arg::with_name("long")
                .short("l")
                .long("long")
                .takes_value(false)
                .help("Long listing"),
        )
        .get_matches();

    Ok(Config {
        paths: matches.values_of_lossy("paths").unwrap(),
        long: matches.is_present("long"),
        show_hidden: matches.is_present("all"),
    })
}

pub fn run(config: Config) -> MyResult<()> {
    let paths = find_files(&config.paths, config.show_hidden)?;
    if config.long {
        println!("{}", format_output(&paths)?);
    } else {
        for path in paths {
            println!("{}", path.display());
        }
    }
    Ok(())
}

fn find_files(paths: &[String], show_hidden: bool) -> MyResult<Vec<PathBuf>> {
    let mut res = vec![];

    for path in paths {
        match fs::metadata(path) {
            Err(e) => eprintln!("{}: {}", path, e),
            Ok(metadata) => {
                if metadata.is_file() {
                    res.push(PathBuf::from(path));
                } else if metadata.is_dir() {
                    for entry in fs::read_dir(path)? {
                        let entry = entry?;
                        let path = entry.path();
                        let is_hidden = path.file_name().map_or(false, |file_name| {
                            file_name.to_string_lossy().starts_with(".")
                        });
                        if !is_hidden || show_hidden {
                            res.push(entry.path());
                        }
                    }
                }
            }
        }
    }

    Ok(res)
}

fn format_output(paths: &[PathBuf]) -> MyResult<String> {
    //               1   2    3    4    5    6    7    8
    let fmt = "{:<}{:<} {:>} {:<} {:<} {:>} {:<} {:<}";
    let mut table = Table::new(fmt);

    for path in paths {
        let meta = path.metadata()?;

        let uid = meta.uid();
        let user = users::get_user_by_uid(uid)
            .map(|u| u.name().to_string_lossy().into_owned())
            .unwrap_or_else(|| uid.to_string());

        let gid = meta.gid();
        let group = users::get_group_by_gid(gid)
            .map(|g| g.name().to_string_lossy().into_owned())
            .unwrap_or_else(|| gid.to_string());

        let file_type = if path.is_dir() { "d" } else { "-" };
        let perms = format_mode(meta.mode());
        let modified: DateTime<Local> = DateTime::from(meta.modified()?);

        table.add_row(
            Row::new()
                .with_cell(file_type) // 1 "d"または"-"
                .with_cell(perms) // 2 パーミッション
                .with_cell(meta.nlink()) // 3 リンク数
                .with_cell(user) // 4 ユーザー名
                .with_cell(group) // 5 グループ名
                .with_cell(meta.len()) // 6 サイズ
                .with_cell(modified.format("%b %d %y %H:%M")) // 7 更新日時
                .with_cell(path.display()), // 8 パス
        );
    }

    Ok(format!("{}", table))
}

fn format_mode(mode: u32) -> String {
    let fmt = |m: usize| -> &str { ["---", "--x", "-w-", "-wx", "r--", "r-x", "rw-", "rwx"][m] };

    let user_mode = (mode as usize >> 6) & 0o7;
    let group_mode = (mode as usize >> 3) & 0o7;
    let other_mode = mode as usize & 0o7;
    format!("{}{}{}", fmt(user_mode), fmt(group_mode), fmt(other_mode))
}

#[cfg(test)]
mod test {
    use super::{find_files, format_mode, format_output};
    use std::path::PathBuf;

    #[test]
    fn test_find_files() {
        // ディレクトリにある隠しエントリ以外のエントリを検索する
        let res = find_files(&["tests/inputs".to_string()], false);
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            [
                "tests/inputs/bustle.txt",
                "tests/inputs/dir",
                "tests/inputs/empty.txt",
                "tests/inputs/fox.txt",
            ]
        );

        // 存在するファイルは、隠しファイルであっても検索できるようにする
        let res = find_files(&["tests/inputs/.hidden".to_string()], false);
        assert!(res.is_ok());
        let filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        assert_eq!(filenames, ["tests/inputs/.hidden"]);

        // 複数のパスを与えてテストする
        let res = find_files(
            &[
                "tests/inputs/bustle.txt".to_string(),
                "tests/inputs/dir".to_string(),
            ],
            false,
        );
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            ["tests/inputs/bustle.txt", "tests/inputs/dir/spiders.txt"]
        );
    }

    #[test]
    fn test_find_files_hidden() {
        // ディレクトリにあるすべてのエントリを検索する
        let res = find_files(&["tests/inputs".to_string()], true);
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            [
                "tests/inputs/.hidden",
                "tests/inputs/bustle.txt",
                "tests/inputs/dir",
                "tests/inputs/empty.txt",
                "tests/inputs/fox.txt",
            ]
        );
    }

    #[test]
    fn test_format_mode() {
        assert_eq!(format_mode(0o755), "rwxr-xr-x");
        assert_eq!(format_mode(0o421), "r---w---x");
    }

    // helper
    fn long_match(
        line: &str,
        expected_name: &str,
        expected_perms: &str,
        expected_size: Option<&str>,
    ) {
        let parts: Vec<_> = line.split_whitespace().collect();
        assert!(parts.len() > 0 && parts.len() <= 10);

        let perms = parts.get(0).unwrap();
        assert_eq!(perms, &expected_perms);

        if let Some(size) = expected_size {
            let file_size = parts.get(4).unwrap();
            assert_eq!(file_size, &size);
        }

        let display_name = parts.last().unwrap();
        assert_eq!(display_name, &expected_name);
    }

    #[test]
    fn test_format_output_one() {
        let bustle_path = "tests/inputs/bustle.txt";
        let bustle = PathBuf::from(bustle_path);

        let res = format_output(&[bustle]);
        assert!(res.is_ok());

        let out = res.unwrap();
        let lines: Vec<&str> = out.split("\n").filter(|s| !s.is_empty()).collect();
        assert_eq!(lines.len(), 1);

        let line1 = lines.first().unwrap();
        long_match(&line1, bustle_path, "-rw-r--r--", Some("193"));
    }

    #[test]
    fn test_format_output_two() {
        let res = format_output(&[
            PathBuf::from("tests/inputs/dir"),
            PathBuf::from("tests/inputs/empty.txt"),
        ]);
        assert!(res.is_ok());

        let out = res.unwrap();
        let mut lines: Vec<&str> = out.split("\n").filter(|s| !s.is_empty()).collect();
        lines.sort();
        assert_eq!(lines.len(), 2);

        let empty_line = lines.remove(0);
        long_match(
            &empty_line,
            "tests/inputs/empty.txt",
            "-rw-r--r--",
            Some("0"),
        );

        let dir_line = lines.remove(0);
        long_match(&dir_line, "tests/inputs/dir", "drwxr-xr-x", None);
    }
}
