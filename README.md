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

## Deployment Setup

### NixOS (recommend)

- Add the Plural Kitty flake `plural-kitty.url = "git+https://codeberg.org/Apothecary/plural-kitty.git";`
- Import the NixOS module `modules = [ plural-kitty.nixosModules.default .. ];`
- Configure Plural Kitty [(see example)](./docs/config-examples/example.nix)
- Configure your HTTP server to forwarded the message send endpoint to Plural Kitty [(example for Nginx)](./docs/config-examples/example-nginx.nix)

### Generic Linux

- Install Plural Kitty and run it as a service (see the [Packaging Guide](#Packaging Guide) below for help)
- Configure Plural Kitty [(see example)](./docs/config-examples/example.yaml)
- Configure your HTTP server to forwarded the message send endpoint to Plural Kitty [(example for Nginx)](./docs/config-examples/example-nginx.conf)

## Packaging Guide

To build Plural Kitty from source you will need:

- rustc v1.70 or later and cargo
- openssl development libraries
- sqlite development libraries

Download the source code then build it by running `cargo build -r`. The
resulting binary will be located at `./target/release/plural-kitty`. Install
the binary at an appropriate location. You should also create a service for
Plural Kitty. An [example Systemd service](./docs/config-examples/example.plural-kitty.service)
is provided.

## Devel Setup

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

