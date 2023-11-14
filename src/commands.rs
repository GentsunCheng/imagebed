use std::fs;
use std::io::{self, Write};

use crate::config::Config;

pub fn clear_storage() {
    let config = Config::from_toml("config/config.toml");
    let www_root = config.www_root();
    let storage_path = format!("{}/file/", www_root);

    let mut count = 0u64;
    let entries = fs::read_dir(&storage_path).unwrap();
    for entry in entries {
        if entry.unwrap().path().is_file() {
            count += 1;
        }
    }

    println!("This command will clear ALL the files in {}. The action is irreversible!", &storage_path);
    println!("There are {} file(s) in total.", count);
    print!("Do you want to proceed? [y/N]");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("error when reading input.");
    let first_char = input.trim().chars().next();
    match first_char {
        Some(c) => {
            if c == 'y' {
                let entries = fs::read_dir(&storage_path).unwrap();
    
                for entry in entries {
                    let path = entry.unwrap().path();
                    if path.is_file() {
                        fs::remove_file(path).unwrap();
                    }
                }

                println!("Removed {} files.", count);
            } else {
                println!("Abort.");
                return;
            }
        },
        None => {
            println!("Abort.");
            return;
        }
    }
}