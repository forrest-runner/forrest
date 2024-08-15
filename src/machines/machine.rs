use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use log::{debug, error, info, warn};
use octocrab::models::actions::SelfHostedRunnerJitConfig;
use octocrab::models::{RunnerGroupId, RunnerId};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use tokio::task::AbortHandle;

use super::manager::{Machines, Rescheduler};
use super::run_dir::RunDir;
use super::triplet::Triplet;
use crate::auth::Auth;
use crate::config::{ConfigFile, MachineConfig};

#[derive(PartialEq, Clone, Copy, Debug)]
pub(super) enum Status {
    Requested,
    Registering,
    Registered,
    Starting,
    Waiting,
    Running,
    Stopping,
    Stopped,
}

/// The mutable part of `Machine`.
/// These are modified when the machine transitiones through the different states.
struct Inner {
    abort: Option<AbortHandle>,
    jit_config: Option<SelfHostedRunnerJitConfig>,
    run_dir: Option<RunDir>,
    started: Option<Instant>,
    status: Status,
}

pub(super) struct Machine {
    auth: Arc<Auth>,
    cfg: Arc<ConfigFile>,
    inner: Mutex<Inner>,
    rescheduler: Rescheduler,
    runner_name: String,
    triplet: Triplet,
}

impl Status {
    /// Is this machine available to process a new job?
    ///
    /// A machine that is already processing a job is not.
    pub(super) fn is_available(&self) -> bool {
        match self {
            Self::Requested
            | Self::Registering
            | Self::Registered
            | Self::Starting
            | Self::Waiting => true,
            Self::Running | Self::Stopping | Self::Stopped => false,
        }
    }

    /// Is this machine in its final done state?
    ///
    /// Machines will not be in this state for long,
    /// because the `machines::Manager` will remove them from the list the very next time
    /// it locks its list of machines.
    pub(super) fn is_stopped(&self) -> bool {
        *self == Self::Stopped
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(match self {
            Self::Requested => "requested",
            Self::Registering => "registering",
            Self::Registered => "registered",
            Self::Starting => "starting",
            Self::Waiting => "waiting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
        })
    }
}

impl Inner {
    fn runner_id(&self) -> Option<RunnerId> {
        self.jit_config.as_ref().map(|jc| jc.runner.id)
    }
}

impl Machine {
    /// Get a new machine in the `Requested` state.
    ///
    /// # Arguments
    ///
    /// * `cfg` - The version of the config file this machine will use throughout
    ///   its lifetime.
    /// * `auth` - The authentication cache we use to register the jit runner with
    ///   GitHub. This has to know about the user in `triplet` already.
    /// * `rescheduler` - Used to trigger a reschedule from the `machines::Manager`
    ///   once the machine exits and its resources are available to other machines.
    /// * `triplet` - The (owner, repository, machine name) triplet that requested
    ///   this machine.
    pub(super) fn new(
        cfg: Arc<ConfigFile>,
        auth: Arc<Auth>,
        rescheduler: Rescheduler,
        triplet: Triplet,
    ) -> Option<Arc<Self>> {
        let machine_config = cfg
            .repositories
            .get(triplet.owner())
            .and_then(|repos| repos.get(triplet.repository()))
            .and_then(|repo| repo.machines.get(triplet.machine_name()));

        if machine_config.is_none() {
            error!("Got request for unknown machine triplet: {triplet}");
            return None;
        }

        let runner_name = {
            // Build a runner name like "forrest-build-rHCiNOhFdypjtnfj"

            let mut name = b"forrest-".to_vec();

            name.extend(triplet.machine_name().as_bytes());
            name.push(b'-');
            name.extend(thread_rng().sample_iter(&Alphanumeric).take(16));

            String::from_utf8(name).unwrap()
        };

        let inner = Mutex::new(Inner {
            status: Status::Requested,
            run_dir: None,
            abort: None,
            jit_config: None,
            started: None,
        });

        Some(Arc::new(Self {
            triplet,
            rescheduler,
            runner_name,
            auth,
            cfg,
            inner,
        }))
    }

    fn inner(&self) -> std::sync::MutexGuard<Inner> {
        self.inner.lock().unwrap()
    }

    /// How much effort went into this machine already?
    ///
    /// When demand for a machine type suddenly drops (because e.g. a run was canceled)
    /// we need to decide which machines to kill.
    /// It makes more sense to kill a machine that is e.g. not yet registered as runner
    /// instead of one that is already booted and waiting for a job.
    pub(super) fn cost_to_kill(&self) -> u32 {
        match self.inner().status {
            Status::Requested => 0,
            Status::Registering => 1,
            Status::Registered => 2,
            Status::Starting => 3,
            Status::Waiting => 4,
            Status::Running | Status::Stopping | Status::Stopped => u32::MAX,
        }
    }

    pub(super) fn cfg(&self) -> &ConfigFile {
        &self.cfg
    }

    pub(super) fn triplet(&self) -> &Triplet {
        &self.triplet
    }

    pub(super) fn machine_config(&self) -> &MachineConfig {
        let cfg = self.cfg();
        let triplet = self.triplet();

        let machine_config = cfg
            .repositories
            .get(triplet.owner())
            .and_then(|repos| repos.get(triplet.repository()))
            .and_then(|repo| repo.machines.get(triplet.machine_name()));

        machine_config.unwrap()
    }

    /// The amount of RAM (in bytes) the machine may currently consume
    pub(super) fn ram_consumed(&self) -> u64 {
        match self.inner().status {
            Status::Requested | Status::Registering | Status::Registered | Status::Stopped => 0,
            Status::Starting | Status::Waiting | Status::Running | Status::Stopping => {
                self.ram_required()
            }
        }
    }

    /// Get the amount of RAM (in bytes) the machine would consume if it were started
    pub(super) fn ram_required(&self) -> u64 {
        self.machine_config().ram.bytes()
    }

    pub(super) fn runner_name(&self) -> &str {
        &self.runner_name
    }

    /// The amount of time the machine has already spent in the starting state
    ///
    /// E.g. the machine was booted but we did not observe it registering as
    /// runner yet via the API.
    pub(super) fn starting_duration(&self) -> Option<Duration> {
        let inner = self.inner();

        match inner.status {
            Status::Starting => inner.started.map(|s| s.elapsed()),
            _ => None,
        }
    }

    pub(super) fn status(&self) -> Status {
        self.inner().status
    }

    /// Register this machine as a JIT GitHub runner
    fn register(self: &Arc<Self>, inner: &mut Inner) {
        assert_eq!(inner.status, Status::Requested);

        let machine = self.clone();

        let task = tokio::spawn(async move {
            let triplet = machine.triplet();
            let installation_octocrab = machine.auth.user(machine.triplet.owner()).unwrap();

            let labels = vec![
                "self-hosted".to_owned(),
                "forrest".to_owned(),
                triplet.machine_name().into(),
            ];

            let runner_group = RunnerGroupId(1);

            let jit_config = installation_octocrab
                .actions()
                .create_repo_jit_runner_config(
                    triplet.owner(),
                    triplet.repository(),
                    &machine.runner_name,
                    runner_group,
                    labels,
                )
                .send()
                .await;

            let mut inner = machine.inner();

            match jit_config {
                Ok(jc) => {
                    debug!(
                        "Registered jit runner for {}: {} {}",
                        machine.triplet, machine.runner_name, jc.runner.id
                    );

                    inner.status = Status::Registered;
                    inner.jit_config = Some(jc);
                }
                Err(err) => {
                    error!(
                        "Failed to register jit runner for {}: {err}",
                        machine.triplet
                    );

                    inner.status = Status::Stopped;
                }
            }

            // The task is about to end.
            // No need to stop it from the outside anymore.
            inner.abort = None;

            // We must release the lock before calling reschedule
            std::mem::drop(inner);
            machine.rescheduler.reschedule();
        });

        inner.status = Status::Registering;
        inner.abort = Some(task.abort_handle());

        println!("Registering {self} as jit runner is not yet implemented");
        println!("Will pretend to do so instead");

        inner.status = Status::Registered;
    }

    // Spawn qemu in the background and keep the machine state updated
    fn spawn(self: &Arc<Self>, inner: &mut Inner) {
        assert_eq!(inner.status, Status::Registered);

        println!("Spawning a VM for {self} is not yet implemented");
        println!("Will pretend to do so instead");

        inner.status = Status::Starting;
        inner.started = Some(Instant::now());

        println!("And now I will pretend it is done");

        // Update our status to stopped and some other cleanup.
        self.kill();

        // Maybe schedule new machines in the space we freed.
        self.rescheduler.reschedule();
    }

    /// Stop this machine, set the status to stopped and maybe de-register the jit runner.
    pub(super) fn kill(self: &Arc<Self>) {
        let mut inner_locked = self.inner();

        println!("No VM was spawned for {self} so we can just pretend to kill it");

        inner_locked.status = Status::Stopped;

        if let Some(runner_id) = inner_locked.runner_id() {
            // We have to de-register the runner

            let machine = self.clone();

            tokio::spawn(async move {
                let octocrab = machine.auth.user(machine.triplet.owner()).unwrap();

                let res = octocrab
                    .actions()
                    .delete_repo_runner(
                        machine.triplet.owner(),
                        machine.triplet.repository(),
                        runner_id,
                    )
                    .await;

                machine.inner().jit_config = None;

                match res {
                    Ok(()) => info!(
                        "De-registered {} on {}",
                        machine.runner_name, machine.triplet
                    ),
                    Err(err) => {
                        warn!(
                            "Failed to de-register {} from {}: {err}",
                            machine.runner_name, machine.triplet
                        )
                    }
                }
            });
        }
    }

    /// Reguest a move of the machine through its state machine
    ///
    /// This either triggers the registration as a jit runner or spawns the qemu process.
    /// Other progress in the state machine is made via `status_feedback`.
    ///
    /// The `ram_available` argument is used to decide if the machine can be spawned
    /// and is updated _if_ the machine was spawned.
    ///
    /// The `machines` argument is checked if the machine this machine is based on is
    /// currently running.
    /// If so the startup of this machine is delayed since a new base image is likely to
    /// be available soon, which should be used instead of the current base image or
    /// the machine image.
    pub(super) fn reschedule(self: &Arc<Self>, ram_available: &mut u64, machines: &Machines) {
        let mut inner = self.inner();

        match inner.status {
            Status::Requested => self.register(&mut inner),
            Status::Registered => {
                let ram_required = self.ram_required();

                if ram_required > *ram_available {
                    debug!("Postpone starting {self} due to insufficient RAM {ram_available} vs. {ram_required}");
                    return;
                }

                let run_dir = RunDir::new(self, machines);

                match run_dir {
                    Ok(run_dir) => inner.run_dir = run_dir,
                    Err(err) => {
                        error!("Failed to set up run dir for {self}: {err}");
                        inner.status = Status::Stopped;
                        return;
                    }
                }

                if inner.run_dir.is_some() {
                    self.spawn(&mut inner);
                    *ram_available -= ram_required;
                }
            }
            Status::Registering
            | Status::Starting
            | Status::Waiting
            | Status::Running
            | Status::Stopping
            | Status::Stopped => {}
        }
    }

    /// Update the state of the machine using feedback from jobs and runner API
    ///
    /// The feedback we get from job states may be able to tell us if the machine
    /// is online (because it could not be processing a job otherwise) but it
    /// can not tell us if the machine is offline, hence the `Option<bool>`.
    pub(super) fn status_feedback(&self, online: Option<bool>, busy: bool) {
        let mut inner = self.inner();

        let new = match (&inner.status, online, busy) {
            // Stay in the current state
            (Status::Requested, _, _) => Status::Requested,
            (Status::Registering, _, _) => Status::Registering,
            (Status::Registered, _, _) => Status::Registered,
            (Status::Starting, Some(false) | None, _) => Status::Starting,
            (Status::Waiting, Some(true) | None, false) => Status::Waiting,
            (Status::Running, Some(true) | None, true) => Status::Running,
            (Status::Stopping, _, _) => Status::Stopping,
            (Status::Stopped, _, _) => Status::Stopped,

            // The action runner on the machine has registered itself
            // but does not run a job yet.
            (Status::Starting, Some(true), false) => Status::Waiting,

            // The action runner has taken up a job
            (Status::Starting | Status::Waiting, _, true) => Status::Running,

            // The job is complete and the machine about to stop
            (Status::Waiting, Some(false), _)
            | (Status::Running, Some(false), _)
            | (Status::Running, _, false) => {
                inner.jit_config = None;

                Status::Stopping
            }
        };

        if inner.status != new {
            info!(
                "Machine {self} transitioned from state {} to {new}",
                inner.status
            );
            inner.status = new;
        }
    }
}

impl std::fmt::Display for Machine {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {}", self.triplet, self.runner_name)
    }
}
