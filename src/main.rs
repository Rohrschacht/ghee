use std::cmp::Reverse;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::ops::{Add, Sub};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, DurationRound, FixedOffset, Local, SecondsFormat};
use clap::{Args, Parser, Subcommand};
use libbtrfsutil as btrfs;
use regex::Regex;
use serde::Deserialize;
use tabled::{Style, Table, Tabled};

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

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct Job {
    subvolume: String,
    target: String,
    groups: Vec<String>,
    preserve: PreservePolicy,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct PreservePolicy {
    retention: String,
    min: PreservePolicyMin,
}

#[derive(Debug, Deserialize)]
enum RetentionPolicy {
    No,
    Policy(String),
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
enum PreservePolicyMin {
    Variant(PreservePolicyMinVariants),
    Timespan(String),
    Count(usize),
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
enum PreservePolicyMinVariants {
    #[serde(alias = "all")]
    All,
    #[serde(alias = "latest")]
    Latest,
}

#[derive(Debug, PartialEq)]
enum IntentType {
    Create,
    Keep,
    Delete,
}

#[derive(Debug, Tabled)]
struct Intent<'a> {
    #[tabled(display_with("Self::display_intent", args))]
    intent: IntentType,
    subvolume: String,
    target: String,
    name: String,
    #[tabled(skip)]
    job: &'a Job,
}

impl<'a> Intent<'a> {
    fn display_intent(&self) -> String {
        match self.intent {
            IntentType::Create => "++++++".to_string(),
            IntentType::Keep => "======".to_string(),
            IntentType::Delete => "------".to_string(),
        }
    }
}

fn filter_jobs(jobs: &[Job], groups: &[String]) -> Vec<Job> {
    let filtered_jobs = if !groups.is_empty() {
        jobs.iter()
            .filter(|j| j.groups.iter().any(|jg| groups.contains(jg)))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        jobs.iter().cloned().collect::<Vec<_>>()
    };

    filtered_jobs
}

#[derive(Debug)]
struct DurationParseError;

impl Display for DurationParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error while parsing Duration from String")
    }
}

impl Error for DurationParseError {}

fn duration_from_str(s: &str) -> Result<Duration, Box<dyn Error>> {
    let re = Regex::new(r"^(?:(\d+)h)?\s*(?:(\d+)d)?\s*(?:(\d+)w)?\s*(?:(\d+)m)?\s*(?:(\d+)y)?$")
        .unwrap();
    let mut d = Duration::zero();

    if !re.is_match(s) {
        return Err(Box::new(DurationParseError));
    }

    let capture = re.captures(s).unwrap();

    let hours = capture.get(1);
    let days = capture.get(2);
    let weeks = capture.get(3);
    let months = capture.get(4);
    let years = capture.get(5);

    println!("{:?}", hours);
    println!("{:?}", days);
    println!("{:?}", weeks);
    println!("{:?}", months);
    println!("{:?}", years);

    if let Some(h) = hours {
        d = d.add(Duration::hours(h.as_str().parse()?));
    }
    if let Some(days) = days {
        d = d.add(Duration::days(days.as_str().parse()?));
    }
    if let Some(w) = weeks {
        d = d.add(Duration::weeks(w.as_str().parse()?));
    }
    if let Some(m) = months {
        d = d.add(Duration::weeks(4 * m.as_str().parse::<i64>()?));
    }
    if let Some(y) = years {
        d = d.add(Duration::days(365 * y.as_str().parse::<i64>()?));
    }

    Ok(d)
}

fn gather_create_intents(jobs: &[Job]) -> Vec<Intent> {
    let now = Local::now();
    let now_str = now.to_rfc3339_opts(SecondsFormat::Secs, true);

    let mut create_intents = Vec::new();
    for job in jobs {
        let subvolume_path = Path::new(&job.subvolume);

        let subvolume_test = btrfs::is_subvolume(&job.subvolume);
        match subvolume_test {
            Err(e) => eprintln!("{} is not a btrfs subvolume! {}", &job.subvolume, e),
            Ok(is_subvol) => {
                if !is_subvol {
                    eprintln!("{} is not a btrfs subvolume!", &job.subvolume);
                } else {
                    create_intents.push(Intent {
                        intent: IntentType::Create,
                        subvolume: job.subvolume.clone(),
                        target: job.target.clone(),
                        name: format!(
                            "{}.{}",
                            subvolume_path.file_name().unwrap().to_str().unwrap(),
                            now_str
                        ),
                        job,
                    });
                }
            }
        }
    }

    create_intents
}

fn gather_delete_intents(jobs: &[Job]) -> Vec<Intent> {
    let mut delete_intents = Vec::new();
    for job in jobs {
        let subvolume_path = Path::new(&job.subvolume);
        let re = format!(
            "{}.{}",
            &subvolume_path.file_name().unwrap().to_str().unwrap(),
            r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}([+-]\d{2}:\d{2})?"
        );
        // println!("{}", re);
        let re = Regex::new(&re).unwrap();

        let paths = fs::read_dir(&job.target).unwrap();
        for path in paths {
            let path = path.unwrap();
            if path.metadata().unwrap().is_dir() {
                if re.is_match(path.file_name().to_str().unwrap()) {
                    // println!("{}", path.file_name().to_str().unwrap());
                    delete_intents.push(Intent {
                        intent: IntentType::Delete,
                        subvolume: job.subvolume.clone(),
                        target: path.path().to_str().unwrap().to_string(),
                        name: path.file_name().to_str().unwrap().to_string(),
                        job,
                    })
                }
            }
        }
    }

    delete_intents
}

fn timestamp_from_intent(intent: &Intent) -> DateTime<FixedOffset> {
    let time_re =
        Regex::new(r".*\.(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}([+-]\d{2}:\d{2})?)").unwrap();
    let timestamp = time_re.captures(&intent.name).unwrap().get(1).unwrap();
    let timestamp = DateTime::parse_from_rfc3339(timestamp.as_str()).unwrap();
    timestamp
}

fn delete_to_keep_intents(intents: &mut Vec<Intent>, jobs: &[Job]) {
    for job in jobs {
        let delete_intents = intents
            .into_iter()
            .filter(|i| i.intent == IntentType::Delete)
            .map(|i| (timestamp_from_intent(&i), i));

        let mut job_intents = delete_intents
            .filter(|t| t.1.job == job)
            .collect::<Vec<_>>();
        job_intents.sort_by_key(|t| Reverse(t.0));
        let job_intents = job_intents.into_iter();

        match &job.preserve.min {
            PreservePolicyMin::Variant(PreservePolicyMinVariants::All) => {
                job_intents.for_each(|t| t.1.intent = IntentType::Keep);
            }
            PreservePolicyMin::Variant(PreservePolicyMinVariants::Latest) => {
                job_intents
                    .take(1)
                    .for_each(|t| t.1.intent = IntentType::Keep);
            }
            PreservePolicyMin::Timespan(ts) => {
                let d = duration_from_str(ts);
                match d {
                    Err(e) => {
                        eprintln!("error while handling preserve min for job: {}\nerror: {}\nfor safety, will not delete any snapshots from this job!", &job.subvolume, e);
                        job_intents.for_each(|t| t.1.intent = IntentType::Keep);
                    }
                    Ok(d) => {
                        println!("{:?}", d);
                        job_intents
                            .take_while(|t| t.0 > Local::now().sub(d))
                            .for_each(|t| t.1.intent = IntentType::Keep)
                    }
                };
            }
            PreservePolicyMin::Count(n) => {
                job_intents
                    .take(n.clone())
                    .for_each(|t| t.1.intent = IntentType::Keep);
            }
        };

        // TODO parse retention policy and set corresponding intents to keep
    }
}

fn print_intents(intents: &[Intent]) {
    let table = Table::new(intents).with(Style::modern()).to_string();
    println!("{}", table);
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

            let filtered_jobs = filter_jobs(&config.jobs, &groups);

            let mut intents = gather_create_intents(&filtered_jobs[..]);
            intents.append(gather_delete_intents(&filtered_jobs[..]).as_mut());
            delete_to_keep_intents(&mut intents, &filtered_jobs[..]);
            println!("{:?}", intents);
            print_intents(&intents);
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
