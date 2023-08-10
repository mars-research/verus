{
  description = "A simple project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }: let
    # System types to support.
    supportedSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
  in flake-utils.lib.eachSystem supportedSystems (system: let
    pkgs = import nixpkgs {
      inherit system;
      overlays = [
        rust-overlay.overlays.default
      ];
    };

    inherit (pkgs) lib;

    pinnedRust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
    pinnedZ3 = pkgs.callPackage ./tools/z3.nix {};
  in {
    devShell = pkgs.mkShell {
      nativeBuildInputs = with pkgs; [
        pinnedRust
        pinnedZ3
      ];

      buildInputs = with pkgs; [
        zlib
      ];

      VERUS_Z3_PATH = lib.getExe pinnedZ3;

      VARGO_TOOLCHAIN = "host";

      # For vstd_build
      RUST_SYSROOT = pinnedRust;

      # For rust_verify
      shellHook = ''
        export LD_LIBRARY_PATH="${pinnedRust}/lib";
      '' + lib.optionalString pkgs.stdenv.isDarwin ''
        export DYLD_LIBRARY_PATH="${pinnedRust}/lib";
      '';
    };
  });
}
