#!/bin/sh

curl -X POST http://localhost:8008/_matrix/client/v3/register \
    -H "Content-Type: application/json" \
    -d '{
        "auth": {
            "type": "m.login.password",
            "type": "m.id.user",
            "identifier": {
                "type": "m.id.user",
                "user": "test"
            },
            "password": "test"
        },
        "device_id": "Setup Script",
        "initial_device_display_name": "Test Account",
        "username": "test",
        "password": "test",
        "kind": "user"
    }'
