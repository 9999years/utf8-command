{
  description = "UTF-8 encoded `std::process::Command` output for Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    systems.url = "github:nix-systems/default";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  nixConfig = {
    extra-substituters = ["https://cache.garnix.io"];
    extra-trusted-substituters = ["https://cache.garnix.io"];
    extra-trusted-public-keys = ["cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g="];
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    systems,
    crane,
    advisory-db,
  }: let
    eachSystem = nixpkgs.lib.genAttrs (import systems);
  in {
    packages = eachSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      inherit (pkgs) lib;
      packages = pkgs.callPackage ./nix/makePackages.nix {inherit inputs;};
    in
      (lib.filterAttrs (name: value: lib.isDerivation value) packages)
      // {
        default = packages.utf8-command;
        docs = packages.utf8-command-docs;
        docs-tarball = packages.utf8-command-docs-tarball;

        # This lets us use `nix run .#cargo` to run Cargo commands without
        # loading the entire `nix develop` shell (which includes
        # `rust-analyzer`).
        #
        # Used in `.github/workflows/release.yaml`.
        cargo = pkgs.cargo;
      });

    checks = eachSystem (system: self.packages.${system}.utf8-command.checks);

    devShells = eachSystem (system: {
      default = self.packages.${system}.utf8-command.devShell;
    });
  };
}
