The Config File
===============

The config file contains some information about the host (the amount of RAM
available for virtual machines and the directory to place things under),
how to authenticate with GitHub,
information about our "machines" and about the repositories Forrest should serve.

Forrest regularly if the config file has changed on disk and will automatically
re-read its content.
Machines that have already been created are not affected by these config reloads
and will use the old config for their entire lifetime from being requested to
stopping.
The authentication keys are also interpreted only once at startup.

Here is an example that uses some (but not all) of the features Forrest has:

```yaml
host:
  base_dir: /srv/forrest
  ram: 120G

github:
  app_id: 1234
  jwt_key_file: key.pem
  polling_interval: 15m
  webhook_secret: Some super secret text

machine_snippets:
  cfg-template: &cfg-template
    setup_template:
      path: /etc/forrest/templates/generic
      parameters:
        RUNNER_VERSION: "2.318.0"
        RUNNER_HASH: "28ed88e4cedf0fc93201a901e392a70463dbd0213f2ce9d57a4ab495027f3e2f"

  os-arch: &os-arch
    base_image: /srv/forrest/images/Arch-Linux-x86_64-cloudimg.img
  os-debian: &os-debian
    base_image: /srv/forrest/images/debian-12-generic-amd64.raw

  machine-small: &machine-small
    cpus: 4
    disk: 16G
    ram: 4G
  machine-medium: &machine-medium
    cpus: 8
    disk: 32G
    ram: 8G

repositories:
  hnez:
    forrest-images:
      persistence_token: <PERSISTENCE_TOKEN>
      machines:
        arch-base:
          << : [*cfg-template, *os-arch, *machine-small]
          use_base: always
        debian-base:
          << : [*cfg-template, *os-debian, *machine-small]
          use_base: always
        debian-yocto:
          << : [*cfg-template, *os-debian, *machine-small]
          base_machine: hnez/forrest-images/debian-base
          use_base: always

    forrest-test:
      machines:
        test-debian:
          << : [*cfg-template, *os-debian, *machine-medium]
          base_machine: hnez/forrest-images/debian-base
```

Config options
--------------

# `host.base_dir`

The directory where Forrest places virtual machine images and other voltatile data.
This directory must be on the same partition as your base virtual machine images
and must use a filesystem with reflink support, like btrfs or xfs.

# `host.ram`

The amount of RAM Forrest is allowed to distribute to virtual machines.
Forrest will spawn as many virtual machines in parallel as it can fit into
this amount of RAM.
Keep in mind that there is some additional overhead per VM and that your
host system also needs some RAM to work.

# `github.app_id`

The id number of your GitHub App.
You have to create a GitHub App in the GitHub developer settings to use with Forrest.

# `github.jwt_key_file`

A path to the `*.private-key.pem` file you get from GitHub when setting up the App.

# `github.webhook_secret`

The webhook secret you have configured in the GitHub App configuration.
This should be a long random string because Forrest will trust any incoming request
that proves that it has access to this webhook secret.

# `github.polling_interval`

(Optional)

Configure how often the GitHub API is polled for updates.
Polling is a backup in case we have missed webhook events and is not a replacenent
for the webhook.
The default interval is 15 minutes and should not be reduced too far.

# `*_snippets`

(Optional)

All top level configuration fields that end in `_snippets` are ignored.
These can be used to create config snippets that can be reused in other
config sections.

# `repositories.<user>.<repository>`

The main section of the configuration file.
The `user` and `repository` are GitHub user names and their repositories.

# `repositories.<user>.<repository>.persistence_token`

(Optional)

Set a persistence token for this repository.
The persistence token should be a long, random string that does however not
contains characters that are problematic when used in a shell script
(because they are used in GitHub workflow files).
You can use e.g. `pwgen -ns 32 1` or `uuidgen -r` to generate them.

The persistence token can be stored as a GitHub workflow secret,
so that is only available to jobs running on a branch,
but not for ones running on pull requests.

After the machine ran Forrest checks if the job has left the persistence token
in a file and if so will make the disk image of said job the new base image
for this machine type.

# `repositories.<user>.<repository>.machines.<machine type>`

Configures a machine that can be used in workflows.
To use e.g. `<machine type>` `build` you would use:

```yaml
jobs:
  example:
    runs-on: [self-hosted, forrest, build]
```

in your workflow file.

# `repositories.<user>.<repository>.machines.<machine type>.base_machine`

(Optional)

Use the machine image of another machine as a basis if no machine image is
available yet for this `<machine type>`
(or if the base machine's image is newer or based on other rules.
See `use_base` for more information).

The format of the base machine is `<owner>/<repository>/<machine_type>`.

> [!NOTE]
> Derived machines will delay their startup until no instances of the machine
> they are based on are running.
> This works around race conditions between a job finishing from GitHub's point
> of view and the machine having stopped successfully and persisted it's disk
> image.

# `repositories.<user>.<repository>.machines.<machine type>.base_image`

(Optional)

Use this image file if not machine image is available yet for this `<machine type>`
(or if the base machine's image is newer or based on other rules.
See `use_base` for more information).

The image file must reside on the same partition as the `host.base_dir`
to enable reflink copies of it.

# `repositories.<user>.<repository>.machines.<machine type>.setup_template.path`

The path to a directory containing `cloud-init` and `job-config` template files.
All files in these directories should be UTF-8 text, because text replacement of
patterns takes place on them to input information like the `<JITCONFIG>`.

An example that is generic enough for use with Debian and Arch Linux images is
provided in the `contrib/setup_templates/generic` directory of the Forrest
repository.

# `repositories.<user>.<repository>.machines.<machine type>.setup_template.parameters`

(Optional)

A mapping of extra parameters to pass through to the `cloud-init` and `job-config`
text replacement logic.

Specifying e.g.:

```yaml
parameters:
  RUNNER_VERSION: "2.318.0"
```

will result in the pattern `<RUNNER_VERSION>` being replaced with `2.318.0` in the
config files.

# `repositories.<user>.<repository>.machines.<machine type>.use_base`

(Optional)

One of:

- `if_newer` (default) - Use the base (e.g. `base_image` or `base_machine`) instead of an
   existing machine image from a previous run if the base is newer than the
   machine image.
- `always` - Always use the base and never reuse a machine image.
  This makes sense for machines that are only used to generate machine images for
  use as base in machines, which should however always do so from scratch.
- `never` - Always run from a previous machine image.

# `repositories.<user>.<repository>.machines.<machine type>.cpu`

The number of virtual CPUs to give to the machine.
There is no limit on how many virtual CPUs are used in parallel.

# `repositories.<user>.<repository>.machines.<machine type>.disk`

The size disk images will be increased to before starting the machine.
The value has to be specified with a suffix of `B`, `K`, `M`, `G` or `T`.

# `repositories.<user>.<repository>.machines.<machine type>.ram`

The amount of RAM to give to this machine.
The value has to be specified with a suffix of `B`, `K`, `M`, `G` or `T`.
Forrest will spawn additional virtual machines until `host.ram` is used up.

# `repositories.<user>.<repository>.machines.<machine type>.shared`

(optional)

A list of directories on the host that should be made available to the guest
using `virtfs`.

> [!NOTE]
> This feature requires `9pfs` support in the virtual machine,
> which as of writing this is not available in the `linux-image-cloud` kernel
> installed by default in `debian-*-genericcloud-amd64.raw` images.
> Use `debian-*-generic-amd64.raw` instead when using this option.

> [!WARNING]
> This option has some security implications,
> especially when setting the `writable` flag.

# `repositories.<user>.<repository>.machines.<machine type>.shared[<N>].path`

The path of the directory on the host.

# `repositories.<user>.<repository>.machines.<machine type>.shared[<N>].tag`

The `virtfs` mount tag to use in the virtual machine.

# `repositories.<user>.<repository>.machines.<machine type>.shared[<N>].writable`

(Optional)

Whether or not to allow writes to the directory.
Defaults to `false`.

> [!WARNING]
> Make absolutely sure you know what you are doing before setting this to `true`.
