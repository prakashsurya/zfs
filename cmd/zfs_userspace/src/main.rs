mod zfsioctl;

use clap::{Parser, Subcommand};
use std::io::Write;
use std::{fs::File, os::fd::AsRawFd};
use nix::unistd::Uid;
use nix::unistd::User;
use strum::IntoEnumIterator;

use crate::zfsioctl::{zfs_pool_configs, zfs_userspace, UserQuotaProp};

use std::collections::HashMap;
use std::collections::BTreeMap;

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

    let file = File::open("/dev/zfs")?;

    match args.command {
        Command::Vdevs { pool, json } => {
            let mut pool_configs = zfs_pool_configs(file.as_raw_fd());
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
            let mut users: BTreeMap<(String, u32), HashMap<UserQuotaProp, u64>> = BTreeMap::new();

            for prop in UserQuotaProp::iter() {
                for useracct in zfs_userspace(file.as_raw_fd(), &dataset, prop) {
                    let user = (useracct.domain, useracct.rid);
        
                    match users.get_mut(&user) {
                        None => {
                            let mut new = HashMap::new();
                            new.insert(prop, useracct.space);
                            users.insert(user, new);
                        },
                        Some(x) => {
                            x.insert(prop, useracct.space);
                        }
                    };
                }
            }
        
            write!(std::io::stdout(), "{:20}", "NAME")?;
            for prop in UserQuotaProp::iter() {
                write!(std::io::stdout(), "{:20}", prop.tostr())?;
            }
            write!(std::io::stdout(), "\n")?;
        
            for ((_domain, rid), props) in users {
                let user = match numeric_id {
                    true => rid.to_string(),
                    false => User::from_uid(Uid::from_raw(rid)).ok().unwrap().unwrap().name
                };
        
                write!(std::io::stdout(), "{:20}", user)?;
        
                for prop in UserQuotaProp::iter() {
                    match props.get(&prop) {
                        None => write!(std::io::stdout(), "{:20}", "-")?,
                        Some(x) => write!(std::io::stdout(), "{:20}", x.to_string())?,
                    }
                }
        
                write!(std::io::stdout(), "\n")?;
            }
        }
    }

    Ok(())
}