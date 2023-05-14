mod parse;
mod prepare;
mod run;
mod types;

use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use crate::parse::parse;
use crate::prepare::prepare;
use crate::run::run;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let file_path = if args.len() > 1 {
        &args[1]
    } else {
        "monty.py"
    };
    let code = match read_file(file_path) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::FAILURE;
        }
    };
    let nodes = parse(&code, None).unwrap();
    // dbg!(&nodes);
    let (namespace_size, nodes) = prepare(nodes).unwrap();
    // dbg!(namespace_size, &nodes);
    let tic = Instant::now();
    match run(namespace_size, &nodes) {
        Ok(_) => {
            let toc = Instant::now();
            eprintln!("Elapsed time: {:?}", toc - tic);
            ExitCode::SUCCESS
        },
        Err(err) => {
            eprintln!("Error running code: {}", err);
            ExitCode::FAILURE
        }
    }
}

fn read_file(file_path: &str) -> Result<String, String> {
    eprintln!("Reading file: {}", file_path);
    match fs::metadata(file_path) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(format!("Error: {file_path} is not a file"));
            }
        }
        Err(err) => {
            return Err(format!("Error reading {file_path}: {err}"));
        }
    }
    match fs::read_to_string(file_path) {
        Ok(contents) => Ok(contents),
        Err(err) => Err(format!("Error reading file: {err}"))
    }
}
