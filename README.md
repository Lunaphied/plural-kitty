# Plural Kitty

A WIP tool that allows plural users to switch aliases without breaking e2ee or
generating a lot of extra events.

The proxy works by sitting in front of synapses message send endpoint. When a plural user
sends a message, the proxy will check if they have the correct name and avatar in that room,
and if they do not, the proxy will set it before the message send is forwarded to synapse.

We concertize that there will be a bot users DM to set up their profiles and control the proxy.

## Devel setup

Requirements:

- Nix

Or:

- Rust 1.70 or newer
- podman
- podman-compose
- pkg-config
- Openssl
- Sqlite
- Nginx
- Postgresql 12 or newer
- (Optional) [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)

Set up:

- Run `nix-shell` in this directory to install all of the needed programs.
- Run `./scripts/start-dev-env.sh` to start start postgres, synapse, and nginx.
- Run `./scripts/setup.sh` to create the development users
- Run `cargo run ./test_server/config.yaml` to run the proxy.
- You will be prompted for the pk account password, it is `kitty`.
- Connect to the test home server at `http://localhost:8000` with the client of your choice
- You can log in as `@test:test.local` with the password `test`.

Ports `8000`, `8008`, `4000`, and `5432` will be used by default,
make sure they're available on your system
