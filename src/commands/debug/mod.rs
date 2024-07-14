use clap::Parser;
use std::io::{BufRead, Write};

use bch_bindgen::bcachefs;
use bch_bindgen::c;
use bch_bindgen::fs::Fs;

mod bkey_types;
mod parser;

use bch_bindgen::c::bpos;

use anyhow::Result;

/// Debug a bcachefs filesystem.
#[derive(Parser, Debug)]
pub struct Cli {
    #[arg(required(true))]
    devices: Vec<std::path::PathBuf>,

    #[arg(short, long)]
    command: Option<String>,
}

#[derive(Debug)]
enum DebugCommand {
    Dump(DumpCommand),
}

#[derive(Debug)]
struct DumpCommand {
    btree: String,
    bpos: bpos,
}

fn dump(fs: &Fs, cmd: DumpCommand) {
    let id: bch_bindgen::c::btree_id = match cmd.btree.parse() {
        Ok(b) => b,
        Err(_) => {
            eprintln!("unknown btree '{}'", cmd.btree);
            return;
        }
    };

    unsafe {
        c::cmd_dump_bkey(fs.raw, id, cmd.bpos);
    }
}

fn usage() {
    println!("Usage:");
    println!("    dump <btree_type> <bpos>");
}

fn do_command(fs: &Fs, cmd: &str) -> i32 {
    match parser::parse_command(cmd) {
        Ok(cmd) => {
            match cmd {
                DebugCommand::Dump(cmd) => dump(fs, cmd),
            };

            0
        }
        Err(e) => {
            println!("{e}");
            usage();

            1
        }
    }
}

pub fn debug(argv: Vec<String>) -> Result<()> {
    fn prompt() {
        print!("bcachefs> ");
        std::io::stdout().flush().unwrap();
    }

    let opt = Cli::parse_from(argv);
    let fs_opts: bcachefs::bch_opts = Default::default();

    if let Some(cmd) = opt.command {
        return match parser::parse_command(&cmd) {
            Ok(cmd) => {
                let fs = Fs::open(&opt.devices, fs_opts)?;
                match cmd {
                    DebugCommand::Dump(cmd) => dump(&fs, cmd),
                }

                Ok(())
            }
            Err(e) => {
                println!("{e}");
                usage();

                Ok(())
            }
        };
    }

    let fs = Fs::open(&opt.devices, fs_opts)?;

    prompt();
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        do_command(&fs, &line.unwrap());
        prompt();
    }

    Ok(())
}

pub fn list_bkeys() -> Result<()> {
    print!("{}", bkey_types::get_bkey_type_info()?);

    Ok(())
}
