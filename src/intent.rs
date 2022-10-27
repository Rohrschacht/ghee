use std::cell::RefCell;
use std::cmp::Reverse;
use std::fs;
use std::ops::Sub;
use std::path::Path;
use std::rc::Rc;

use chrono::{DateTime, FixedOffset, Local, SecondsFormat};
use libbtrfsutil as btrfs;
use log::{debug, error, info, trace, warn};
use regex::Regex;
use tabled::{Style, Table, Tabled};

use crate::duration::duration_from_str;
use crate::executed_intent::ExecutedIntent;
use crate::job::Job;
use crate::policies::{PreservePolicyMin, PreservePolicyMinVariants};
use crate::retention::Retention;
use crate::timebins::TimeBins;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IntentType {
    Create,
    Keep,
    Delete,
}

#[derive(Debug, Tabled, Clone)]
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

    pub fn print_tabled(intents: &[Rc<RefCell<Self>>]) {
        let intents = intents
            .iter()
            .map(|r| (*r.borrow()).clone())
            .collect::<Vec<_>>();
        let table = Table::new(intents).with(Style::modern()).to_string();
        info!("{}", table);
    }

    pub fn execute(&self) -> ExecutedIntent {
        match self.intent {
            IntentType::Create => {
                let res = btrfs::create_snapshot(
                    &self.subvolume,
                    &format!("{}/{}", self.target, self.name),
                    btrfs::CreateSnapshotFlags::READ_ONLY,
                    None,
                );
                match res {
                    Ok(_) => ExecutedIntent::new(self, true),
                    Err(e) => {
                        warn!("creating snapshot failed! error: {}", e);
                        ExecutedIntent::new(self, false)
                    }
                }
            }
            IntentType::Keep => ExecutedIntent::new(self, true),
            IntentType::Delete => {
                let res =
                    btrfs::delete_subvolume(&self.target, btrfs::DeleteSubvolumeFlags::empty());
                match res {
                    Ok(_) => ExecutedIntent::new(self, true),
                    Err(e) => {
                        warn!("deleting snapshot failed! error: {}", e);
                        ExecutedIntent::new(self, false)
                    }
                }
            }
        }
    }

    pub fn gather_create_intents(jobs: &'a [Job]) -> Vec<Rc<RefCell<Self>>> {
        let now = Local::now();
        let now_str = now.to_rfc3339_opts(SecondsFormat::Secs, true);

        let mut create_intents = Vec::new();
        for job in jobs {
            let subvolume_path = Path::new(&job.subvolume);

            let subvolume_test = btrfs::is_subvolume(&job.subvolume);
            match subvolume_test {
                Err(e) => warn!("{} is not a btrfs subvolume! Error: {}", &job.subvolume, e),
                Ok(is_subvol) => {
                    if !is_subvol {
                        warn!(
                            "{} is not a btrfs subvolume! Can't create a snapshot of it!",
                            &job.subvolume
                        );
                    } else {
                        create_intents.push(Rc::new(RefCell::new(Intent {
                            intent: IntentType::Create,
                            subvolume: job.subvolume.clone(),
                            target: job.target.clone(),
                            name: format!(
                                "{}.{}",
                                subvolume_path.file_name().unwrap().to_str().unwrap(),
                                now_str
                            ),
                            job,
                        })));
                    }
                }
            }
        }

        create_intents
    }

    pub fn gather_delete_intents(jobs: &'a [Job]) -> Vec<Rc<RefCell<Self>>> {
        let mut delete_intents = Vec::new();
        for job in jobs {
            let subvolume_path = Path::new(&job.subvolume);
            let re = format!(
                "{}.{}",
                &subvolume_path.file_name().unwrap().to_str().unwrap(),
                r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}([+-]\d{2}:\d{2})?"
            );
            let re = Regex::new(&re).unwrap();

            let paths = fs::read_dir(&job.target);
            match paths {
                Err(e) => error!("Unable to read directory {}! Error: {}", &job.target, e),
                Ok(paths) => {
                    for path in paths {
                        match path {
                            Err(e) => error!(
                                "IO error occured when accessing {}! Error: {}",
                                &job.target, e
                            ),
                            Ok(path) => match path.metadata() {
                                Err(e) => error!(
                                    "Unable to read metadata of {:?}! Error: {}",
                                    path.path(),
                                    e
                                ),
                                Ok(metadata) => {
                                    if metadata.is_dir() {
                                        match path.file_name().to_str() {
                                            None => error!(
                                                "Unable to parse Unicode from path {:?}!",
                                                path.path()
                                            ),
                                            Some(filename) => {
                                                if re.is_match(filename) {
                                                    delete_intents.push(Rc::new(RefCell::new(
                                                        Intent {
                                                            intent: IntentType::Delete,
                                                            subvolume: job.subvolume.clone(),
                                                            target: path
                                                                .path()
                                                                .to_str()
                                                                .unwrap()
                                                                .to_string(),
                                                            name: path
                                                                .file_name()
                                                                .to_str()
                                                                .unwrap()
                                                                .to_string(),
                                                            job,
                                                        },
                                                    )));
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }

        delete_intents
    }

    pub fn delete_to_keep_intents(intents: &mut [Rc<RefCell<Self>>], jobs: &[Job]) {
        for job in jobs {
            let delete_intents = intents
                .iter_mut()
                .filter(|int| int.borrow().intent == IntentType::Delete)
                .map(|int| (int.borrow().timestamp(), Rc::clone(int)));

            let mut job_intents = delete_intents
                .filter(|(_ts, int)| int.borrow().job == job)
                .collect::<Vec<_>>();
            job_intents.sort_by_key(|t| Reverse(t.0));
            let job_intents = job_intents.into_iter();

            match &job.preserve.min {
                PreservePolicyMin::Variant(PreservePolicyMinVariants::All) => {
                    job_intents
                        .for_each(|(_ts, int)| (*int).borrow_mut().intent = IntentType::Keep);
                }
                PreservePolicyMin::Variant(PreservePolicyMinVariants::Latest) => {
                    job_intents
                        .take(1)
                        .for_each(|(_ts, int)| (*int).borrow_mut().intent = IntentType::Keep);
                }
                PreservePolicyMin::Timespan(ts) => {
                    let d = duration_from_str(ts);
                    match d {
                        Err(e) => {
                            warn!("error while handling preserve min for job: {}\nerror: {}\nfor safety, will not delete any snapshots from this job!", &job.subvolume, e);
                            job_intents.for_each(|(_ts, int)| {
                                (*int).borrow_mut().intent = IntentType::Keep
                            });
                        }
                        Ok(d) => {
                            debug!("parsed duration for preserve min: {:?}", d);
                            job_intents
                                .take_while(|(ts, _int)| ts > &Local::now().sub(d))
                                .for_each(|(_ts, int)| {
                                    (*int).borrow_mut().intent = IntentType::Keep
                                })
                        }
                    };
                }
                PreservePolicyMin::Count(n) => {
                    job_intents
                        .take(*n)
                        .for_each(|(_ts, int)| (*int).borrow_mut().intent = IntentType::Keep);
                }
            };

            // parse retention policy and set corresponding intents to keep
            let delete_intents = intents
                .iter_mut()
                .filter(|int| int.borrow().intent == IntentType::Delete)
                .map(|int| (int.borrow().timestamp(), Rc::clone(int)));

            let mut job_intents = delete_intents
                .filter(|(_ts, int)| int.borrow().job == job)
                .collect::<Vec<_>>();
            job_intents.sort_by_key(|t| Reverse(t.0));
            let job_intents = job_intents.into_iter();

            let retention = Retention::from_str(&job.preserve.retention);
            match retention {
                Err(e) => {
                    warn!("error while handling preserve retention for job: {}\nerror: {}\nfor safety, will not delete any snapshots from this job!", &job.subvolume, e);
                    job_intents
                        .for_each(|(_ts, int)| (*int).borrow_mut().intent = IntentType::Keep);
                }
                Ok(retention) => {
                    let mut timebins = TimeBins::new(&retention);

                    debug!("timebins before filling: {:?}", timebins);

                    for (timestamp, intent) in job_intents {
                        timebins.store(&timestamp, Rc::clone(&intent));
                    }

                    debug!("timebins after filling: {:?}", timebins);

                    timebins.set_keep();
                }
            };
        }
    }
}
