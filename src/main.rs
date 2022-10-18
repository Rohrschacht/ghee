use std::fs;

use clap::{Parser, Subcommand};
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
    let args = Cli::parse();
    println!("{:?}", args);

    let config = fs::read_to_string(args.config)?;
    println!("{}", config);
    let config: Config = serde_yaml::from_str(&config)?;
    println!("{:?}", config);

    match args.command {
        Commands::Dryrun { groups } => {
            println!("Will dry run with groups: {:?}", groups);

            let filtered_jobs = Job::filter_active_groups(&config.jobs, &groups);

            let mut intents = Intent::gather_create_intents(&filtered_jobs[..]);
            intents.append(Intent::gather_delete_intents(&filtered_jobs[..]).as_mut());
            Intent::delete_to_keep_intents(&mut intents, &filtered_jobs[..]);
            println!("{:?}", intents);
            Intent::print_tabled(&intents);
        }
        Commands::Prune { groups } => {
            println!("Will dry run with groups: {:?}", groups);
        }
        Commands::Run { groups } => {
            println!("Will dry run with groups: {:?}", groups);
        }
    }

    Ok(())
}
