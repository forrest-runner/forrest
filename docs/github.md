GitHub Application Setup
========================

Forrest is designed to authenticate with the GitHub API as an App,
because they provide a non-expiring authentication method.

You can create a new GitHub App in the
[developer settings](https://github.com/settings/apps).

You need to:

- Generate a client secret to use for authentication later.
- Provide a webhook URL _with a secret_ that terminates at a reverse proxy
  on the host running Forrest.
  See [Setting up nginx](nginx.md) for an example on how to
  configure nginx as reverse proxy for Forrest.
- Enable Read and Write "Actions", "Administration" (to add jit runners)
  and "Contents" repository permissions for the app.
- Enable "Workflow job" events for the app.
- Install the app for your user/app.
