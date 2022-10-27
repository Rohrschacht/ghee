use serde::Deserialize;

use crate::policies::PreservePolicy;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Job {
    pub subvolume: String,
    pub target: String,
    pub groups: Option<Vec<String>>,
    pub preserve: PreservePolicy,
}

impl Job {
    pub fn filter_active_groups(jobs: &[Self], groups: &[String]) -> Vec<Self> {
        let filtered_jobs = if !groups.is_empty() {
            jobs.iter()
                .filter(|j| j.groups.is_some())
                .filter(|j| {
                    j.groups
                        .as_ref()
                        .unwrap()
                        .iter()
                        .any(|jg| groups.contains(jg))
                })
                .cloned()
                .collect::<Vec<_>>()
        } else {
            jobs.to_vec()
        };

        filtered_jobs
    }
}
