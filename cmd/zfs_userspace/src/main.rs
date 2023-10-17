mod zfsioctl;

use clap::{Parser, Subcommand};
use std::io::Write;
use std::{fs::File, os::fd::AsRawFd};

use crate::zfsioctl::{zfs_pool_configs, zfs_userspace, UserQuotaProp};

#[derive(Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Vdevs {
        pool: Option<String>,

        #[arg(short = 'j')]
        json: bool,
    },

    UserSpace {
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
    },
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let _file = File::open("/dev/zfs")?;

    match args.command {
        Command::Vdevs { pool, json } => {
            let mut pool_configs = zfs_pool_configs(_file.as_raw_fd());
            if let Some(p) = pool {
                if let Some(config) = pool_configs.find(|config| config.name == p) {
                    match json {
                        true => {
                            let json = serde_json::to_string_pretty(&config.vdevs)?;
                            println!("{json}");
                        }
                        false => {
                            for vdev in config.vdevs {
                                match vdev.is_log {
                                    true => println!("{} (log)", vdev.path),
                                    false => println!("{}", vdev.path),
                                }
                            }
                        }
                    }
                }
            } else {
                for config in pool_configs {
                    match json {
                        true => {
                            let json = serde_json::to_string_pretty(&config)?;
                            println!("{json}");
                        }
                        false => {
                            println!("{}:", config.name);
                            for vdev in config.vdevs {
                                match vdev.is_log {
                                    true => println!("\t{} (log)", vdev.path),
                                    false => println!("\t{}", vdev.path),
                                }
                            }
                        }
                    }
                }
            }
        }
        Command::UserSpace {
            prtnum,
            scripted,
            parseable,
            ofield,
            tfield,
            ifield,
            dataset,
        } => {
            let prop = UserQuotaProp::UserUsed;
            for useracct in zfs_userspace(_file.as_raw_fd(), &dataset, prop) {
                //writeln!(std::io::stdout(), "{useracct:?}")?;
                writeln!(
                    std::io::stdout(),
                    "{}: {}",
                    useracct.name_string(prop.name_type(), !prtnum),
                    useracct.space,
                )?;
            }
        }
    }
    Ok(())
}
