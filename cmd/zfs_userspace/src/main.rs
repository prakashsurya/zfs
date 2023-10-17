use clap::Parser;
use std::fs::File;

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

    Ok(())
}
