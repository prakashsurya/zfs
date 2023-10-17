mod zfsioctl;

use clap::Parser;
use std::io::Write;
use std::{fs::File, os::fd::AsRawFd};

use crate::zfsioctl::zfs_userspace;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short = 'n')]
    prtnum: bool,

    #[arg(short = 'H')]
    scripted: bool,

    #[arg(short = 'p')]
    parseable: bool,

    #[arg(short = 'o')]
    ofield: Option<String>,

    // TODO: Add '-S' and '-s' for sorting.
    #[arg(short = 't')]
    tfield: Option<String>,

    #[arg(short = 'i')]
    ifield: bool,

    #[arg(index = 1)]
    dataset: String,
}

fn main() -> std::io::Result<()> {
    let _args = Args::parse();
    println!("Args: {:?}", _args);

    let _file = File::open("/dev/zfs")?;

    println!("Hello, world!");

    for useracct in zfs_userspace(
        _file.as_raw_fd(),
        &_args.dataset,
        zfsioctl::UserQuotaProp::UserUsed,
    ) {
        writeln!(std::io::stdout(), "{useracct:?}")?;
    }

    Ok(())
}
