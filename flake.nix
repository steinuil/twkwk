{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naerskPackage.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, nixpkgs, flake-utils, naerskPackage }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) { inherit system; };

        naersk = pkgs.callPackage naerskPackage { };
      in
      {
        defaultPackage = naersk.buildPackage {
          src = ./.;
        };

        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rustc
            cargo
          ];

          buildInputs = with pkgs; [
            rust-analyzer
            clippy
            lldb
            rustfmt
          ];
        };
      }
    );
}

