use std::cmp::Reverse;
use std::fs;
use std::ops::Sub;
use std::path::Path;

use chrono::{DateTime, FixedOffset, Local, SecondsFormat};
use libbtrfsutil as btrfs;
use regex::Regex;
use tabled::{Style, Table, Tabled};

use crate::duration::duration_from_str;
use crate::job::Job;
use crate::policies::{PreservePolicyMin, PreservePolicyMinVariants};
use crate::retention::Retention;
use crate::timebins::TimeBins;

#[derive(Debug, PartialEq)]
pub enum IntentType {
    Create,
    Keep,
    Delete,
}

#[derive(Debug, Tabled)]
pub struct Intent<'a> {
    #[tabled(display_with("Self::display_intent", args))]
    pub intent: IntentType,
    pub subvolume: String,
    pub target: String,
    pub name: String,
    #[tabled(skip)]
    pub job: &'a Job,
}

impl<'a> Intent<'a> {
    fn display_intent(&self) -> String {
        match self.intent {
            IntentType::Create => "++++++".to_string(),
            IntentType::Keep => "======".to_string(),
            IntentType::Delete => "------".to_string(),
        }
    }

    pub fn timestamp(&self) -> DateTime<FixedOffset> {
        let time_re =
            Regex::new(r".*\.(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}([+-]\d{2}:\d{2})?)").unwrap();
        let timestamp = time_re.captures(&self.name).unwrap().get(1).unwrap();
        let timestamp = DateTime::parse_from_rfc3339(timestamp.as_str()).unwrap();
        timestamp
    }

    pub fn print_tabled(intents: &[Self]) {
        let table = Table::new(intents).with(Style::modern()).to_string();
        println!("{}", table);
    }

    pub fn gather_create_intents(jobs: &'a [Job]) -> Vec<Self> {
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

    pub fn gather_delete_intents(jobs: &'a [Job]) -> Vec<Self> {
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

    pub fn delete_to_keep_intents(intents: &mut Vec<Self>, jobs: &[Job]) {
        for job in jobs {
            let delete_intents = intents
                .into_iter()
                .filter(|int| int.intent == IntentType::Delete)
                .map(|int| (int.timestamp(), int));

            let mut job_intents = delete_intents
                .filter(|(_ts, int)| int.job == job)
                .collect::<Vec<_>>();
            job_intents.sort_by_key(|t| Reverse(t.0));
            let job_intents = job_intents.into_iter();

            match &job.preserve.min {
                PreservePolicyMin::Variant(PreservePolicyMinVariants::All) => {
                    job_intents.for_each(|(_ts, int)| int.intent = IntentType::Keep);
                }
                PreservePolicyMin::Variant(PreservePolicyMinVariants::Latest) => {
                    job_intents
                        .take(1)
                        .for_each(|(_ts, int)| int.intent = IntentType::Keep);
                }
                PreservePolicyMin::Timespan(ts) => {
                    let d = duration_from_str(ts);
                    match d {
                        Err(e) => {
                            eprintln!("error while handling preserve min for job: {}\nerror: {}\nfor safety, will not delete any snapshots from this job!", &job.subvolume, e);
                            job_intents.for_each(|(_ts, int)| int.intent = IntentType::Keep);
                        }
                        Ok(d) => {
                            println!("{:?}", d);
                            job_intents
                                .take_while(|(ts, _int)| ts > &Local::now().sub(d))
                                .for_each(|(_ts, int)| int.intent = IntentType::Keep)
                        }
                    };
                }
                PreservePolicyMin::Count(n) => {
                    job_intents
                        .take(n.clone())
                        .for_each(|(_ts, int)| int.intent = IntentType::Keep);
                }
            };

            // parse retention policy and set corresponding intents to keep
            let delete_intents = intents
                .into_iter()
                .filter(|int| int.intent == IntentType::Delete)
                .map(|int| (int.timestamp(), int));

            let mut job_intents = delete_intents
                .filter(|(_ts, int)| int.job == job)
                .collect::<Vec<_>>();
            job_intents.sort_by_key(|t| Reverse(t.0));
            let job_intents = job_intents.into_iter();

            let retention = Retention::from_str(&job.preserve.retention);
            match retention {
                Err(e) => {
                    eprintln!("error while handling preserve retention for job: {}\nerror: {}\nfor safety, will not delete any snapshots from this job!", &job.subvolume, e);
                    job_intents.for_each(|(_ts, int)| int.intent = IntentType::Keep);
                }
                Ok(retention) => {
                    let timebinned_intents = job_intents
                        .map(|(ts, _int)| TimeBins::new(&ts))
                        .chain([TimeBins::oldest()])
                        .collect::<Vec<_>>();

                    let timebins_current_and_next = timebinned_intents
                        .iter()
                        .zip(timebinned_intents.iter().skip(1))
                        .enumerate();

                    let hourly_indexes = timebins_current_and_next
                        .clone()
                        .filter(|(_i, bins)| bins.0.h != bins.1.h)
                        .map(|(i, _bins)| i)
                        .take(retention.h)
                        .collect::<Vec<_>>();
                    let daily_indexes = timebins_current_and_next
                        .clone()
                        .filter(|(_i, bins)| bins.0.d != bins.1.d)
                        .map(|(i, _bins)| i)
                        .take(retention.d)
                        .collect::<Vec<_>>();
                    let weekly_indexes = timebins_current_and_next
                        .clone()
                        .filter(|(_i, bins)| bins.0.w != bins.1.w)
                        .map(|(i, _bins)| i)
                        .take(retention.w)
                        .collect::<Vec<_>>();
                    let monthly_indexes = timebins_current_and_next
                        .clone()
                        .filter(|(_i, bins)| bins.0.m != bins.1.m)
                        .map(|(i, _bins)| i)
                        .take(retention.m)
                        .collect::<Vec<_>>();
                    let yearly_indexes = timebins_current_and_next
                        .clone()
                        .filter(|(_i, bins)| bins.0.y != bins.1.y)
                        .map(|(i, _bins)| i)
                        .take(retention.y)
                        .collect::<Vec<_>>();

                    println!("{:?}", hourly_indexes);
                    println!("{:?}", daily_indexes);
                    println!("{:?}", weekly_indexes);
                    println!("{:?}", monthly_indexes);
                    println!("{:?}", yearly_indexes);

                    let mut delete_intents = intents
                        .into_iter()
                        .filter(|i| i.intent == IntentType::Delete)
                        .map(|i| (i.timestamp(), i))
                        .collect::<Vec<_>>();
                    delete_intents.sort_by_key(|t| Reverse(t.0));
                    let mut delete_intents = delete_intents
                        .into_iter()
                        .map(|(_ts, int)| int)
                        .collect::<Vec<_>>();

                    for i in hourly_indexes {
                        delete_intents[i].intent = IntentType::Keep;
                    }
                    for i in daily_indexes {
                        delete_intents[i].intent = IntentType::Keep;
                    }
                    for i in weekly_indexes {
                        delete_intents[i].intent = IntentType::Keep;
                    }
                    for i in monthly_indexes {
                        delete_intents[i].intent = IntentType::Keep;
                    }
                    for i in yearly_indexes {
                        delete_intents[i].intent = IntentType::Keep;
                    }
                }
            };
        }
    }
}
