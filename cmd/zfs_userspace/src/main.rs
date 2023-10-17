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
        /// Display numeric uid/gid values instead of names.
        #[arg(long, short = 'n')]
        numeric_id: bool,

        /// Do not print headers, use tab-delimited fields.
        #[arg(long, short = 'H')]
        no_headers: bool,

        /// Display values in parsable (exact) values.
        #[arg(long, short = 'p')]
        parseable: bool,

        /// Display the comma-separated list of fields.
        #[arg(short = 'o')]
        fields: Option<String>,

        /// Display the comma-separated list of entity types (from: all,posixuser,smbuser,posixgroup,smbgroup)
        #[arg(short = 't')]
        types: Option<String>,

        /// Translate SID to POSIX uid/gid.
        #[arg(long, short = 'i')]
        posix_sid: bool,

        /// Sort by the given field.  May be specified more than once.
        #[arg(short = 's')]
        sort: Vec<String>,

        /// Sort by the given field in reverse order.  May be specified more than once.
        #[arg(short = 'S')]
        sort_reverse: Vec<String>,

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
            numeric_id,
            no_headers: scripted,
            parseable,
            fields,
            types,
            posix_sid,
            dataset,
            sort,
            sort_reverse,
        } => {
            let prop = UserQuotaProp::UserUsed;
            for useracct in zfs_userspace(_file.as_raw_fd(), &dataset, prop) {
                //writeln!(std::io::stdout(), "{useracct:?}")?;
                let space = if parseable {
                    useracct.space.to_string()
                } else {
                    format!("{:.1} GB", useracct.space as f64 / 1024.0 / 1024.0 / 1024.0)
                };
                writeln!(
                    std::io::stdout(),
                    "{}: {}",
                    useracct.name_string(prop.name_type(), !numeric_id),
                    space,
                )?;
            }
        }
    }
    Ok(())
}
