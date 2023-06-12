#!/bin/sh

podman-compose up -d
mkdir -p /tmp/nginx-dev
nginx -c "$(pwd)/test_server/nginx.conf" -e "/tmp/nginx-dev/error.log"
