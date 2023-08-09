# Plural Kitty

A WIP tool that allows plural users to switch aliases without breaking e2ee or
generating a lot of extra events.

Plural Kitty works by sitting in front of Synapses message send endpoint. When a plural user
sends a message, the proxy will check if they have the correct name and avatar in that room,
and if they do not, the proxy will set it before the message send is forwarded to synapse.

Plural Kitty also has a bot that users DM to set up and change members.

## Status

Plural Kitty is still very much a work in progress and should be considered alpha software.
You can see a list of current major [TODO items here](./TODO.md). If you have any ideas to improve
Plural Kitty or any questions or issues, please ask about them in 
[#plural-kitty-dev:the-apothecary.club](https://matrix.to/#/#plural-kitty-dev:the-apothecary.club).

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
- [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)

Set up:

- Run `nix-shell` in this directory to install all of the needed programs.
- Run `./scripts/start-dev-env.sh` to start start postgres, synapse, and nginx.
- Run `./scripts/setup.sh` to create the development users
- Run `cargo run ./test_server/config.yaml` to run the proxy.
- You will be prompted for the pk account password, it is `kitty`.
- Connect to the test home server at `http://localhost:8000` with the client of your choice
- You can log in as `@test:test.local` with the password `test`.

Ports `8000`, `8008`, `4000`, and `5432` will be used by default,
make sure they're available on your system.

Also considered using out git hooks by running `ln -s ./.git_hooks ./.git/hooks`.
These hooks will ensure everything stays up to date during development.

