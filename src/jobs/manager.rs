use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use octocrab::models::workflows::Status;
use octocrab::models::{JobId, RunId};
use tokio::task::JoinHandle;

use super::job::Job;
use crate::machines::{Manager as MachineManager, OwnerAndRepo, OwnerRepoMachine};

// The `status_feedback()` method is called for each webhook event
// and each job that comes up in a poll.
// If we start requesting machines as the jobs trickle in,
// then we can not have a policy on when to start which machine.
// Hence give jobs some time to trickle in before updating demand with
// the machine manager.
const UPDATE_SOON_DELAY: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct Manager {
    machine_manager: MachineManager,
    jobs: Arc<Mutex<Vec<Job>>>,
    update_soon_task: Arc<Mutex<JoinHandle<()>>>,
}

impl Manager {
    pub fn new(machine_manager: MachineManager) -> Self {
        let jobs = Arc::new(Mutex::new(Vec::new()));

        // A placeholder task that finishes immediately.
        // Later an actual task will be placed in this spot.
        let update_soon_task = Arc::new(Mutex::new(tokio::spawn(async {})));

        Self {
            machine_manager,
            jobs,
            update_soon_task,
        }
    }

    /// Get GitHub workflow run ids for which we are interested in updates.
    ///
    /// This more or less means all runs with jobs that are not known to
    /// have completed or failed yet.
    pub fn runs_of_interest(&self) -> HashMap<OwnerAndRepo, HashSet<RunId>> {
        let mut res: HashMap<OwnerAndRepo, HashSet<RunId>> = HashMap::new();

        for job in self.jobs.lock().unwrap().iter() {
            if job.is_interesting() {
                let oar = job.orm().clone().into_owner_and_repo();
                let run_id = job.run_id();

                res.entry(oar).or_default().insert(run_id);
            }
        }

        res
    }

    /// Update the status of a job
    ///
    /// This is called by the poller and webhook ingres tasks.
    pub fn status_feedback(
        &self,
        orm: &OwnerRepoMachine,
        job_id: JobId,
        run_id: RunId,
        status: Status,
        runner_name: Option<&str>,
    ) {
        if let (Status::InProgress, Some(runner_name)) = (&status, runner_name) {
            // We know that the runner this job is running on must be online and busy,
            // even though that information may not have trickled through yet.
            // Make sure the runner does not become eligible for termination.
            self.machine_manager
                .status_feedback(orm, runner_name, Some(true), true);
        }

        if let (Status::Completed | Status::Failed, Some(runner_name)) = (&status, runner_name) {
            // We know that the runner this job is running on is no longer busy.
            // We do however not know if it is still online.
            self.machine_manager
                .status_feedback(orm, runner_name, None, false);
        }

        let mut jobs = self.jobs.lock().unwrap();

        let index = jobs
            .iter()
            .position(|job| job.orm() == orm && job.job_id() == job_id);

        let has_changed = match (&status, index) {
            // Track the status of this job by either adding it to our index
            // or updating its state if we already know it.
            (Status::Pending | Status::Queued | Status::InProgress, None) => {
                jobs.push(Job::new(orm.clone(), job_id, run_id, status));
                true
            }
            (Status::Pending | Status::Queued | Status::InProgress, Some(index)) => {
                jobs[index].update_status(status)
            }

            // The job does not need further tracking from our side.
            (Status::Completed | Status::Failed, None) => false,
            (Status::Completed | Status::Failed, Some(index)) => {
                jobs.swap_remove(index);
                true
            }

            // The status enum is marked as non-exhaustive,
            // so we have to have this wildcard match even though all current
            // cases are covered.
            _ => panic!("Got unexpected workflow status from octocrab"),
        };

        if has_changed {
            self.update_demand_soon();
        }
    }

    /// Schedule telling the machine manager how many machines we need
    ///
    /// When a workflow is started it may kick of multiple jobs at once.
    /// We do however not get all the webhook events at once, but one after
    /// the other.
    /// We do however have a bit of a heuristic of which jobs to schedule
    /// first, so we want to wait for all jobs to trickle in before starting
    /// any machines.
    fn update_demand_soon(&self) {
        let mut task = self.update_soon_task.lock().unwrap();

        if !task.is_finished() {
            return;
        }

        let manager = self.clone();

        *task = tokio::spawn(async move {
            tokio::time::sleep(UPDATE_SOON_DELAY).await;
            manager.update_demand();
        });
    }

    /// Tell the machine manager how many machines of which kind we need
    fn update_demand(&self) {
        let jobs = self.jobs.lock().unwrap();

        let orms = jobs
            .iter()
            .filter_map(|job| job.is_queued().then_some(job.orm()));

        self.machine_manager.update_demand(orms);
    }
}
