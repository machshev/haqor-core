{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {nixpkgs, ...}: let
    system = "x86_64-linux";
    #       ↑ Swap it for your system if needed
    #       "aarch64-linux" / "x86_64-darwin" / "aarch64-darwin"
    pkgs = nixpkgs.legacyPackages.${system};
  in {
    devShells.${system}.default = pkgs.mkShell {
        packages = [
            pkgs.cargo
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
  };
}

