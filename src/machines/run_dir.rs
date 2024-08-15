use std::fs::{create_dir_all, File};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use log::{debug, error, info, warn};
use reflink_copy::reflink;

use crate::config::SeedBasePolicy;

use super::machine::Machine;
use super::manager::Machines;

pub(super) struct RunDir {
    run_dir: PathBuf,
}

fn not_found_none<V>(res: std::io::Result<V>) -> std::io::Result<Option<V>> {
    match res {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Pick one of two paths `a` and `b`
///
/// - Pick the one with the more recent modified date if both files exist.
/// - Pick `b` if it exists but `a` does not.
/// - Otherwise pick `a`, regardless of it existing or not.
fn pick_newer<'p>(a: &'p Path, b: &'p Path) -> std::io::Result<&'p Path> {
    let modified_a = not_found_none(a.metadata().and_then(|meta| meta.modified()))?;
    let modified_b = not_found_none(b.metadata().and_then(|meta| meta.modified()))?;

    match (modified_a, modified_b) {
        (Some(ma), Some(mb)) => Ok(if ma > mb { a } else { b }),
        (None, Some(_)) => Ok(b),
        (Some(_), None) | (None, None) => Ok(a),
    }
}

impl RunDir {
    /// Create a directory for a machine run and populate it to match our qemu arguments
    ///
    /// This means placing a `disk.img` file in it to boot from,
    /// a `cloud-init.img` that contains cloud-init configuration and
    /// a `job-config.img` file that contains configuration for running the current job
    /// and is used for feedback from the machine after completion.
    ///
    /// The disk file is based either on a previous run of this machine,
    /// a previous run of another machine (a base machine that generates images)
    /// or a seed file (a plain and unconfigured operating system image).
    ///
    /// Returns Ok(None) if the image file we want is not present yet.
    pub(super) fn new(machine: &Machine, machines: &Machines) -> std::io::Result<Option<Self>> {
        let triplet = machine.triplet();
        let cfg = machine.cfg();
        let machine_config = machine.machine_config();

        let base_dir = &cfg.host.base_dir;

        let machine_image = triplet.machine_image_path(base_dir);

        let base_image = match &machine_config.base_machine {
            Some(base_triplet) if machines.contains_key(base_triplet) => {
                info!("Delaying the startup of {machine} because its base {base_triplet} is currently running");
                return Ok(None);
            }
            Some(base_triplet) => base_triplet.machine_image_path(base_dir),
            None => match &machine_config.base_image {
                Some(base_image) => base_image.clone(),
                None => {
                    warn!("Neither `base_machine` nor `base_image` configured for {machine}.");
                    warn!("Falling back to machine image");
                    machine_image.clone()
                }
            },
        };

        let image = match machine_config.use_base {
            SeedBasePolicy::IfNewer => pick_newer(&base_image, &machine_image)?,
            SeedBasePolicy::Always => &base_image,
            SeedBasePolicy::Never => &machine_image,
        };

        if !image.try_exists()? {
            info!(
                "Delaying the startup of {machine} because the image {} does not exist (yet)",
                image.display()
            );
            return Ok(None);
        }

        let run_dir = triplet.run_dir_path(&cfg.host.base_dir, machine.runner_name());

        create_dir_all(&run_dir)?;

        let disk = run_dir.join("disk.img");

        // Create a copy on write copy of the disk image using reflink
        reflink(image, &disk)?;

        // Grow the disk image if required
        let target_disk_size = machine_config.disk.bytes();
        let current_disk_size = disk.metadata()?.len();

        if current_disk_size < target_disk_size {
            let disk_file = File::options().append(true).open(&disk)?;
            disk_file.set_len(target_disk_size)?;
        }

        let dir = Self { run_dir };

        Ok(Some(dir))
    }
}

impl Drop for RunDir {
    fn drop(&mut self) {
        // Remove the disk file, because it takes up by far the most space.
        // The config files are also removed by their respective drop handler,
        // but e.g. the log files qemu writes will not be deleted,
        // as well as the run dir itself, because they take up little space and
        // may be useful for debugging failed jobs and machines.

        let disk = self.run_dir.join("disk.img");
        let ds = disk.display();

        match std::fs::remove_file(&disk) {
            Ok(()) => debug!("Removed disk file {ds}"),
            Err(e) if e.kind() == ErrorKind::NotFound => {
                debug!("Disk file {ds} was already removed")
            }
            Err(e) => error!("Failed to remove disk image {ds}: {e}"),
        }
    }
}
