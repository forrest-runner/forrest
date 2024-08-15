Debugging a running job
=======================

All (running) jobs have a `shell.sock` unix domain socket in their run directory
(e.g. in `[FORREST ENV PATH]/runs/[USER]/[REPO]/[RUNNER_NAME]`)
that can be used to log into the machine using e.g. `socat`:

```bash
$ socat -,rawer,escape=0x1d UNIX-CONNECT:.../shell.sock
```

> [!NOTE]
> You need to press enter to get an initial prompt
