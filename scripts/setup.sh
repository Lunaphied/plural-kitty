#!/bin/sh

podman-compose exec synapse register_new_matrix_user -c /cfg/homeserver.yaml -u test -p test -a  
podman-compose exec synapse register_new_matrix_user -c /cfg/homeserver.yaml -u pk -p kitty --no-admin
