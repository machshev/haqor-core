{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay/stable";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs:
    with inputs;
      flake-utils.lib.eachDefaultSystem (
        system: let
          overlays = [(import rust-overlay)];
          pkgs = import nixpkgs {
            inherit system overlays;
          };

          # SemVer bump helper: bump-version <major|minor|patch|X.Y.Z> [--tag]
          bump-version = pkgs.writeShellApplication {
            name = "bump-version";
            runtimeInputs = with pkgs; [gnugrep gnused gawk coreutils git];
            text = builtins.readFile ./scripts/bump-version.sh;
          };

          # Keep the CLI installable on its own, while also exposing the
          # workspace crates as buildable package outputs for downstream
          # flakes.
          mkHaqorCrate = crate: pname:
            pkgs.rustPlatform.buildRustPackage {
              inherit pname;
              version = "0.6.1";
              src = ./.;
              cargoLock.lockFile = ./Cargo.lock;
              cargoBuildFlags = ["-p" crate];
              cargoTestFlags = ["-p" crate];
              # The workspace test profile deliberately exercises every
              # morphology paradigm and is too expensive for package builds.
              doCheck = false;
            };

          haqor = mkHaqorCrate "haqor-cli" "haqor";
          haqor-sync-server = mkHaqorCrate "haqor-sync-server" "haqor-sync-server";
        in {
          packages = {
            inherit bump-version haqor haqor-sync-server;
            haqor-cli = haqor;
            haqor-admin = mkHaqorCrate "haqor-admin" "haqor-admin";
            haqor-core = mkHaqorCrate "haqor-core" "haqor-core";
            haqor-db-gen = mkHaqorCrate "haqor-db-gen" "haqor-db-gen";
            haqor-morphology = mkHaqorCrate "haqor-morphology" "haqor-morphology";
          };

          apps.bump-version = {
            type = "app";
            program = "${bump-version}/bin/bump-version";
          };

          apps.haqor = {
            type = "app";
            program = "${haqor}/bin/haqor";
          };

          apps.sync-server = {
            type = "app";
            program = "${haqor-sync-server}/bin/haqor-sync-server";
          };

          devShells = {
            default = with pkgs;
              mkShell {
                buildInputs = [
                  (rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
                  cargo-nextest
                  cargo-udeps
                  cargo-vet
                  cargo-about
                  cargo-release

                  bump-version

                  rust-analyzer
                  rustfmt

                  adrs
                  typos

                  sqlitebrowser

                  # If the dependencies need system libs, you usually need pkg-config + the lib
                  pkg-config
                  openssl
                  sqlite
                ];
              };
          };

          formatter = nixpkgs.legacyPackages.${system}.alejandra;
        }
      );
}
