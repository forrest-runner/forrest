The Job Files
=============

With the [config](../config.md) complete we can write our first workflow files.

Image Generation
----------------

You can either base you Forrest machines directly on an image
provided by your Linux Distribution,
or use an extra workflow to first generate base machine images from them
that are already updated and have required software pre-installed.

Both approached (always basing machines on plain images or generating
base machines) are perfectly valid and which one to choose depends
on your situation.

In this example we generate images for use in other machines.

The workflow file to generate debian bases images
(taken from [hnez/forrest-images](https://github.com/hnez/forrest-images/)):

```yaml
# .github/workflows/debian.yaml
name: Debian based machines

on:
  push:
    branches:
      - main

jobs:
  base:
    name: Base
    runs-on: [self-hosted, forrest, debian-base]
    steps:
      - name: Set up runner machine
        run: |
          sudo localectl set-locale en_US.UTF-8
          export DEBIAN_FRONTEND=noninteractive
          export DPKG_FORCE=confnew
          sudo -E apt-get update
          sudo -E apt-get --assume-yes dist-upgrade
          sudo -E apt-get --assume-yes install git
          sudo -E apt-get --assume-yes autoremove
          sudo -E apt-get --assume-yes clean
      - name: Persist the disk image
        env:
          PERSISTENCE_TOKEN: ${{ secrets.PERSISTENCE_TOKEN }}
        if: ${{ env.PERSISTENCE_TOKEN != ''  }}
        run: |
          sudo fstrim /
          echo "$PERSISTENCE_TOKEN" > ~/config/persist

  yocto:
    name: Yocto
    needs: base
    runs-on: [self-hosted, forrest, debian-yocto]
    steps:
      - name: Install Software
        run: |
          sudo localectl set-locale en_US.UTF-8
          export DEBIAN_FRONTEND=noninteractive
          export DPKG_FORCE=confnew
          sudo -E apt-get update
          sudo -E apt-get --assume-yes dist-upgrade
          sudo -E apt-get --assume-yes install build-essential chrpath diffstat gawk git lz4
          sudo -E apt-get --assume-yes autoremove
          sudo -E apt-get --assume-yes clean
      - name: Persist the disk image
        env:
          PERSISTENCE_TOKEN: ${{ secrets.PERSISTENCE_TOKEN }}
        if: ${{ env.PERSISTENCE_TOKEN != ''  }}
        run: |
          sudo fstrim /
          echo "$PERSISTENCE_TOKEN" > ~/config/persist
```

Differences between public GitHub runners and running on Forrest:

- Jobs specify which `<machine type>` to use via the `runs-on` parameter.
- The images provided by e.g. Debian are very minimal and un-configured
  and may be out of date, hence why the jobs set the locale perform an
  update and have to install basic software like `git`.
- Machine images can be persistend and reused in later runs via the
  `PERSISTENCE_TOKEN`.

Not that the `yocto` job is based on `base` in two ways:

- Via the Forrest config file, which specifies `hnez/forrest-images/debian-base`
  as the `base_machine` for `debian-yocto`.
- Via the `needs: base` entry in the job file.

Build Jobs
----------

When using pre-generated images as a basis we do not have to do the setup
steps and can also decide not to persist the machine images in the end
(always running from the base image instead) hence why a minimal job
does not contain much:

```yaml
# .github/workflows/debian.yaml
name: Use a pre-generated image

on: [push, pull_request]

jobs:
  base:
    name: Base
    runs-on: [self-hosted, forrest, test-debian]
    steps:
      - name: Hello world
        run: echo "Hi from Forrest!"
```
