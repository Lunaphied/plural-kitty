{
  description = "Plural Kitty";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    sqlx-nixpkgs.url = "nixpkgs/e1ee359d16a1886f0771cc433a00827da98d861c";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, sqlx-nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          pkgs-sqlx = sqlx-nixpkgs.legacyPackages.${system};
        in
        {
          devShells.default = import ./shell.nix { inherit pkgs; inherit pkgs-sqlx; };
          packages.default = pkgs.callPackage ./default.nix { };
        }
      ) // {
        nixosModules.default = import ./module.nix self;
      };
}
