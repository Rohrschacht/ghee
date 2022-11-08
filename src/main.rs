use std::fs;
use std::io::Write;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use clap_verbosity_flag::InfoLevel;
use log::{debug, info};
use serde::Deserialize;

use crate::error::ConfigfileExtensionError;
use crate::executed_intent::ExecutedIntent;
use crate::intent::Intent;
use crate::job::Job;

mod duration;
mod error;
mod executed_intent;
mod intent;
mod job;
mod policies;
mod retention;
mod timebins;

/// Automated btrfs snapshots
#[derive(Debug, Parser)]
#[clap(name = "ghee")]
#[clap(about = "Automated btrfs snapshots", long_about = None)]
struct Cli {
    #[clap(short, long, default_value = "/etc/ghee/ghee.yaml")]
    config: String,
    /// Dry run, don't perform any actions
    #[clap(short = 'n', long, default_value = "false")]
    dryrun: bool,
    #[clap(subcommand)]
    command: Commands,
    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity<InfoLevel>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Runs the configured jobs, creates and prunes snapshots
    #[clap(arg_required_else_help = false)]
    Run {
        /// Group filter for configured jobs
        #[clap(value_parser)]
        groups: Vec<String>,
    },
    /// Prints the actions that would be taken
    #[clap(arg_required_else_help = false)]
    Dryrun {
        /// Group filter for configured jobs
        #[clap(value_parser)]
        groups: Vec<String>,
    },
    /// Prunes snapshots
    #[clap(arg_required_else_help = false)]
    Prune {
        /// Group filter for configured jobs
        #[clap(value_parser)]
        groups: Vec<String>,
    },
}

#[derive(Debug, Deserialize)]
struct Config {
    jobs: Vec<Job>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Cli = Cli::parse();
    debug!("program arguments: {:?}", args);

    env_logger::Builder::new()
        .format(|buf, record| {
            if record.level() == log::Level::Info {
                writeln!(buf, "{}", record.args())
            } else {
                writeln!(buf, "[{}] - {}", record.level(), record.args())
            }
        })
        .filter_level(args.verbose.log_level_filter())
        .init();

    let config = fs::read_to_string(&args.config)?;
    debug!("configuration content:\n{}", config);

    let filepath = PathBuf::from(&args.config);
    let fileextension = filepath.extension().ok_or(ConfigfileExtensionError)?;
    let filetype = fileextension.to_str().ok_or(ConfigfileExtensionError)?;
    let config: Config = match filetype {
        "yaml" | "yml" => serde_yaml::from_str(&config)?,
        "json" => serde_json::from_str(&config)?,
        "toml" => toml::from_str(&config)?,
        &_ => return Err(Box::new(ConfigfileExtensionError)),
    };

    debug!("parsed configuration: {:?}", config);

    match args.command {
        Commands::Dryrun { groups } => {
            info!("Will perform a dry run without executing the intents.");
            debug!("Will dry run with groups: {:?}", groups);

            let filtered_jobs = Job::filter_active_groups(&config.jobs, &groups);
            debug!("jobs filtered using active groups: {:?}", filtered_jobs);

            let mut intents = Intent::gather_create_intents(&filtered_jobs[..]);
            intents.append(Intent::gather_delete_intents(&filtered_jobs[..]).as_mut());
            Intent::delete_to_keep_intents(&mut intents, &filtered_jobs[..]);

            debug!("raw intents: {:?}", intents);
            Intent::print_tabled(&intents);
        }
        Commands::Prune { groups } => {
            debug!("Will prune with groups: {:?}", groups);
            info!("Actions that will be performed:");

            let filtered_jobs = Job::filter_active_groups(&config.jobs, &groups);
            debug!("jobs filtered using active groups: {:?}", filtered_jobs);

            let mut intents = Intent::gather_delete_intents(&filtered_jobs[..]);
            Intent::delete_to_keep_intents(&mut intents, &filtered_jobs[..]);

            debug!("raw intents: {:?}", intents);
            Intent::print_tabled(&intents);

            if !args.dryrun {
                let executed_intents = intents.into_iter().map(|i| i.borrow().execute()).collect::<Vec<_>>();
                ExecutedIntent::print_tabled(&executed_intents);
            }
        }
        Commands::Run { groups } => {
            debug!("Will run with groups: {:?}", groups);
            info!("Actions that will be performed:");

            let filtered_jobs = Job::filter_active_groups(&config.jobs, &groups);
            debug!("jobs filtered using active groups: {:?}", filtered_jobs);

            let mut intents = Intent::gather_create_intents(&filtered_jobs[..]);
            intents.append(Intent::gather_delete_intents(&filtered_jobs[..]).as_mut());
            Intent::delete_to_keep_intents(&mut intents, &filtered_jobs[..]);

            debug!("raw intents: {:?}", intents);
            Intent::print_tabled(&intents);

            if !args.dryrun {
                let executed_intents = intents.into_iter().map(|i| i.borrow().execute()).collect::<Vec<_>>();
                ExecutedIntent::print_tabled(&executed_intents);
            }
        }
    }

    Ok(())
}
