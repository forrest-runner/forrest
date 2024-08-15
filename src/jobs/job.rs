use octocrab::models::workflows::Status;
use octocrab::models::{JobId, RunId};

use crate::machines::Triplet;

pub(super) struct Job {
    triplet: Triplet,
    job_id: JobId,
    run_id: RunId,
    status: Status,
}

impl Job {
    pub(super) fn new(triplet: Triplet, job_id: JobId, run_id: RunId, status: Status) -> Self {
        Self {
            triplet,
            job_id,
            run_id,
            status,
        }
    }

    pub(super) fn triplet(&self) -> &Triplet {
        &self.triplet
    }

    pub(super) fn job_id(&self) -> JobId {
        self.job_id
    }

    pub(super) fn run_id(&self) -> RunId {
        self.run_id
    }

    pub(super) fn is_queued(&self) -> bool {
        matches!(self.status, Status::Queued)
    }

    pub(super) fn is_interesting(&self) -> bool {
        match &self.status {
            Status::Pending | Status::Queued | Status::InProgress => true,
            Status::Completed | Status::Failed => false,
            _ => panic!("Got unexpected job status from octocrab"),
        }
    }

    pub(super) fn update_status(&mut self, status: Status) -> bool {
        if self.status != status {
            self.status = status;
            true
        } else {
            false
        }
    }
}
