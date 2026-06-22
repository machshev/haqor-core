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
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };

          # SemVer bump helper: bump-version <major|minor|patch|X.Y.Z> [--tag]
          bump-version = pkgs.writeShellApplication {
            name = "bump-version";
            runtimeInputs = with pkgs; [ gnugrep gnused gawk coreutils git ];
            text = builtins.readFile ./scripts/bump-version.sh;
          };

        in {

          packages.bump-version = bump-version;

          apps.bump-version = {
            type = "app";
            program = "${bump-version}/bin/bump-version";
          };

          devShells = {
            default = with pkgs; mkShell {
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
