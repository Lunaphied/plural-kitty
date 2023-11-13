{ rustPlatform
, git
, binutils
, pkg-config
, openssl
, sqlite
}:

rustPlatform.buildRustPackage rec {
  pname = "plural-kitty";
  version = "0.0.0";
  src = ./.;
  nativeBuildInputs = [ git binutils pkg-config ];
  buildInputs = [ openssl sqlite ];
  cargoLock = {
    lockFile = "${src}/Cargo.lock";
    allowBuiltinFetchGit = true;
  };
}
