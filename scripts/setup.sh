#!/bin/sh

podman-compose exec dendrite create-account -config /etc/dendrite/dendrite.yaml -username test -password testmeow -admin
podman-compose exec dendrite create-account -config /etc/dendrite/dendrite.yaml -username pk -password kittymeow
sqlx database setup
