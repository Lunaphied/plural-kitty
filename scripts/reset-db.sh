#!/bin/sh

sqlx database reset -y && ./scripts/ps < ./test_server/data.sql
