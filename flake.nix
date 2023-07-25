{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      rec {
        packages = rec {
          plural-kitty = pkgs.rustPlatform.buildRustPackage rec {
            pname = name;
            version = "0.1.0";
            src = ./.;
            nativeBuildInputs = with pkgs; [ git binutils ];
            cargoLock = {
              lockFile = "${src}/Cargo.lock";
            };
          };
					default = plural-kitty;
        };

        nixosModules = rec {
          plural-kitty = import ./nixos packages.plural-kitty;
          default = plural-kitty;
        };
      });
}
