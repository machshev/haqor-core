{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    cargo2nix = {
      url = "github:cargo2nix/cargo2nix/release-0.11.0";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
      inputs.rust-overlay.follows = "rust-overlay";
    };
  };

  outputs = inputs:
    with inputs;
      flake-utils.lib.eachDefaultSystem (
        system: let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [cargo2nix.overlays.default];
          };

          rustPkgs = pkgs.rustBuilder.makePackageSet {
            rustVersion = "1.82.0";
            packageFun = import ./Cargo.nix;
          };

          # The workspace defines a development shell with all of the dependencies
          # and environment settings necessary for a regular `cargo build`
          workspaceShell = rustPkgs.workspaceShell {
            # This adds cargo2nix to the project shell via the cargo2nix flake
            packages = [
              cargo2nix.packages."${system}".cargo2nix
              pkgs.cargo
              pkgs.cargo-nextest
              pkgs.cargo-vet
              pkgs.rustc

              pkgs.rust-analyzer
              pkgs.rustfmt

              pkgs.adrs

              # If the dependencies need system libs, you usually need pkg-config + the lib
              pkgs.pkg-config
              pkgs.openssl
            ];

            env = {
              RUST_BACKTRACE = "full";
            };
          };
        in rec {
          packages = {
            # replace hello-world with your package name
            haqor = rustPkgs.workspace.haqor-core {};
            default = packages.haqor;
          };

          devShells = {
            default = workspaceShell; # nix develop
          };

          apps = rec {
            haqor = {
              type = "app";
              program = "${packages.default}/bin/haqor";
            };
            default = haqor;
          };

          formatter = nixpkgs.legacyPackages.${system}.alejandra;
        }
      );
}
