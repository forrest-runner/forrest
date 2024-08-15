Forrest - A GitHub Action Runner Runner
=======================================

                                        ┏━━━━━━━━━━━━━━━┓
                                        ┃      Run      ┃
                                        ┃    Forrest    ┃
                                        ┃      Run      ┃
                                        ┗━━━┯━━━━━━━┯━━━┛

> Now you wouldn’t believe me if I told you, but I could run like the wind blows.<br/>
> From that day on, if I was goin’ somewhere, I was runnin’!

Forrest takes a single host computer and uses it to run multiple virtual
machines with GitHub action runners in them.

The Gist
--------

The virtual machine disk images used are ephermal and removed after a run
completes, but may optionally be persisted to enable pre-generation of images
or to speed up builds.

The following diagram illustrates the possible evolution of an image:

```
src    machine    run
 ╻
 ┣━━━━━━━━━━━━━━━━━┓
 ┃                 ┠─╴Start of a build job for the main branch of the repository.
 ┃                 ┃  The disk image is copied using a reflink copy to create a
 ┃                 ┃  fork of the image that the job can read and write to without
 ┃                 ┃  affecting the src image.
 ┃                 ┃
 ┃                 ┠─╴A virtual machine is started that uses the forked image
 ┃                 ┃  as disk image.
 ┋                 ┋
 ┃                 ┠─╴The job succeeds and provides the correct PERSISTENCE_TOKEN
 ┃                 ┃  for this repository from its GitHub secrets.
 ┃        ┏━━━━━━━━┛
 ┃        ┠──────────╴The disk image left behind by the job becomes the new
 ┃        ┃           base image for this machine.
 ┃        ┃
 ┃        ┣━━━━━━━━┓
 ┃        ┃        ┠─╴Start of a build job for a pull request.
 ┃        ┃        ┃  The job can use everything the previous run left behind to
 ┃        ┃        ┃  speed up the build process.
 ┋        ┋        ┋
 ┃        ┃        ┠─╴The job succeeds but can not provide the PERSISTENCE_TOKEN,
 ┃        ┃        ┃  because secrets are not available to runs on pull requests.
 ┃        ┃        ┃
 ┃        ┃        ┞─╴The disk image for this job is removed.
 ┃        ┃
 ┋        ┋
```
 
The different stages an image can be in are:

- `src` - A base image, these can for example be provided by a Linux Distribution.
  It is also possible to use a machine image from _another_ machine as a base image,
  but we will get to that later in the documentation.
- `machine` - A base image for a run of a specific machine type.
  We will get to what a machine type _is_ later in the documentation as well,
  but for now it just means that later runs will use this image as a base
  instead of `src`.
- `run` - The ephermal virtual machine disk image used in a run.
  The image may be persisted at the end of a run, but could also just be thrown
  away.

Documentation
-------------

The documentation is split into multiple files:

1) Acquiring operating system images suitable for use with Forrest:

   - [Debian](docs/debian-images.md)
   - [Arch Linux](docs/arch-images.md)

2) [Registering a GitHub App for Forrest](docs/github.md)
3) [Writing a Forrest Config File](docs/config.md)
4) [Configuring nginx as Reverse Proxy](docs/nginx.md)
5) [Writing Workflow Jobs using Forrest](docs/jobs.md)
6) [Debugging Machines](docs/debugging.md)

---

                                        ┏━━━━━━━━━━━━━━┓
                                        ┃     Stop     ┃
                                        ┃    Forrest   ┃
                                        ┃     Stop     ┃
                                        ┗━━━┯━━━━━━┯━━━┛

