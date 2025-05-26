Forrest - A GitHub Action Runner Runner
=======================================

                                        ┏━━━━━━━━━━━━━━━┓
                                        ┃      Run      ┃
                                        ┃    Forrest    ┃
                                        ┃      Run      ┃
                                        ┗━━━┯━━━━━━━┯━━━┛

> Now you wouldn’t believe me if I told you, but I could run like the wind blows.<br/>
> From that day on, if I was goin’ somewhere, I was runnin’!

Forrest talks to the GitHub API and spawns a virtual machine using QEMU/KVM
whenever a runner is required to run an action.

This means Forrests helps you to:

  - Run actions on your own hardware, either because you need more powerful
    machines, want to run actions in your own network or want to cache
    artifacts locally.
  - Isolate action runs from one another and the hardware they are running on,
    allowing you to e.g. run actions for public repositories with pull requests
    from the community.

Example Users
-------------

### Forrest Machine Image Generation

- [pengutronix/forrest-images](https://github.com/pengutronix/forrest-images)

  Used at Pengutronix to derive Forrest machine images for Debos, PTXDist and
  Yocto from basic Debian cloud images.

### Yocto Board Support Packages

- [labgrid-project/meta-labgrid](https://github.com/labgrid-project/meta-labgrid/)
- [linux-automation/meta-lxatac](https://github.com/linux-automation/meta-lxatac/)
- [pengutronix/meta-ptx](https://github.com/pengutronix/meta-ptx/)
- [rauc/meta-rauc-community](https://github.com/rauc/meta-rauc-community/)
- [rauc/meta-rauc](https://github.com/rauc/meta-rauc/)

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

