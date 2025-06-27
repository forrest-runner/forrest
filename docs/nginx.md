nginx Setup
===========

GitHub relies on webhooks to deliver realtime updates to Apps like Forrest.
This means we need a publicly reachable web server that exposes a webhook
endpoint.

Forrest delegates the public webserver part of this to a reverse proxy running
on the same host and only provides a unix domain socket to connect to on webhook
events.

Currently `nginx` is the only reverse proxy tested with Forrest.

In your nginx config under the `server` section you should add a proxy directive
to forward requests to Forrest:

```
server {
    listen 443 ssl http2 default_server;
    listen [::]:443 ssl http2 default_server;

    ...

    location /webhook {
        proxy_pass http://unix:[ABSOLUTE PATH TO YOUR FORREST ENV]/api.sock:/webhook;
        proxy_http_version 1.1;
    }
}
```

Replace `[ABSOLUTE PATH TO YOUR FORREST ENV]` with the appropriate path.
