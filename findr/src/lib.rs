use crate::EntryType::*;
use clap::{App, Arg};
use regex::Regex;
use std::error::Error;
use walkdir::WalkDir;

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Eq, PartialEq)]
enum EntryType {
    Dir,
    File,
    Link,
}

#[derive(Debug)]
pub struct Config {
    paths: Vec<String>,
    names: Vec<Regex>,
    entry_types: Vec<EntryType>,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("findr")
        .version("0.1.0")
        .author("Fukkatsuso <fukkatsuso.git+github@gmail.com>")
        .about("Rust find")
        .arg(
            Arg::with_name("paths")
                .value_name("PATH")
                .help("Search paths")
                .multiple(true)
                .default_value("."),
        )
        .arg(
            Arg::with_name("names")
                .short("n")
                .long("name")
                .value_name("NAME")
                .help("Name")
                .multiple(true),
        )
        .arg(
            Arg::with_name("type")
                .short("t")
                .long("type")
                .value_name("TYPE")
                .help("Entry type")
                .possible_values(&["f", "d", "l"])
                .multiple(true),
        )
        .get_matches();

    let names = matches
        .values_of_lossy("names")
        .map(|vals| {
            vals.into_iter()
                .map(|name| Regex::new(&name).map_err(|_| format!("Invalid --name \"{}\"", name)))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();

    let entry_types = matches
        .values_of_lossy("type")
        .map(|vals| {
            vals.iter()
                .map(|t| match t.as_str() {
                    "f" => File,
                    "d" => Dir,
                    "l" => Link,
                    _ => unreachable!("Invalid type"),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Config {
        paths: matches.values_of_lossy("paths").unwrap(),
        names: names,
        entry_types: entry_types,
    })
}

pub fn run(config: Config) -> MyResult<()> {
    for path in config.paths {
        for entry in WalkDir::new(path) {
            match entry {
                Err(e) => eprintln!("{}", e),
                Ok(entry) => {
                    // filtering by type
                    let type_ok = config.entry_types.is_empty()
                        || (config.entry_types.contains(&Dir) && entry.file_type().is_dir())
                        || (config.entry_types.contains(&File) && entry.file_type().is_file())
                        || (config.entry_types.contains(&Link) && entry.file_type().is_symlink());

                    // filtering by name
                    let name_ok = config.names.is_empty()
                        || config
                            .names
                            .iter()
                            .any(|regex| match entry.path().file_name() {
                                Some(name) => regex.is_match(name.to_str().unwrap()),
                                None => false,
                            });

                    if type_ok && name_ok {
                        println!("{}", entry.path().display());
                    }
                }
            }
        }
    }
    Ok(())
}
