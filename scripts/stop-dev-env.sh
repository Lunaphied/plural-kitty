#!/bin/sh

podman-compose down
kill "$(cat /tmp/nginx-dev/nginx.pid)"
