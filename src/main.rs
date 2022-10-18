use std::fs;

use clap::{Parser, Subcommand};
use clap_verbosity_flag::WarnLevel;
use log::{debug, error, info, trace, warn};
use serde::Deserialize;

use crate::intent::Intent;
use crate::job::Job;

mod duration;
mod error;
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
    #[clap(subcommand)]
    command: Commands,
    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity<WarnLevel>,
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

    simple_logger::SimpleLogger::new()
        .with_level(args.verbose.log_level_filter())
        .with_colors(true)
        .without_timestamps()
        .init()
        .unwrap();

    let config = fs::read_to_string(args.config)?;
    debug!("configuration content:\n{}", config);
    let config: Config = serde_yaml::from_str(&config)?;
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
        }
        Commands::Run { groups } => {
            debug!("Will run with groups: {:?}", groups);
        }
    }

    Ok(())
}
