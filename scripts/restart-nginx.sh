#!/bin/sh

kill "$(cat /tmp/nginx-dev/nginx.pid)"
nginx -c "$(pwd)/test_server/nginx.conf" -e "/tmp/nginx-dev/error.log"
