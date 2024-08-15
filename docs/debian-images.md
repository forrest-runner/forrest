Debian Machine Images
=====================

The Debian project provides virtual machine disk images suitable for use
with Forrest.
They can be downloaded from [cdimage.debian.org/images/cloud][cdimage].
The correct images to use are the `*-generic-amd64.raw` variants,
e.g. `debian-12-generic-amd64.raw`.

```bash
$ curl --location --compressed -o debian-12-generic-amd64.raw \
  https://cdimage.debian.org/images/cloud/bookworm/latest/debian-12-generic-amd64.raw
```

[cdimage]: https://cdimage.debian.org/images/cloud/
