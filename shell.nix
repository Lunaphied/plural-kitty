{ pkgs ? import <nixpkgs> { }
, pkgs-sqlx ? import <nixpkgs>
}:

pkgs.mkShell {
  packages = with pkgs; [
    cargo
    rustc
    rust-analyzer
    rustfmt
    clippy
    pkgs-sqlx.sqlx-cli

    pkg-config
    openssl
    sqlite

    postgresql_12
    nginx
    podman
    podman-compose
  ];
  RUST_BACKTRACE = 1;
  PK_LOG = "plural_kitty=debug,warn";
  PGPASSWORD = "beepboop";
  DATABASE_URL = "postgres://synapse:beepboop@localhost/plural_kitty";
}
