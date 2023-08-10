# Nix packages with incremental builds

{ lib
, stdenv
, craneLib
, nix-gitignore
, makeWrapper
, rust
, zlib
, z3
, darwin
, version ? "unstable"
}:

let
  gitignoreRecursiveSource = nix-gitignore.gitignoreFilterSourcePure (_: _: true);
  src = gitignoreRecursiveSource [] ./.;

  commonArgs = {
    inherit version src;
  };

  vargoArgs = commonArgs // {
    cargoLock = ./tools/vargo/Cargo.lock;
    cargoToml = ./tools/vargo/Cargo.toml;

    postUnpack = ''
      cd $sourceRoot/tools/vargo
      sourceRoot="."
    '';
  };

  vargo = craneLib.buildPackage (vargoArgs // {
    cargoArtifacts = craneLib.buildDepsOnly vargoArgs;
  });

  verus = craneLib.buildPackage {
    pname = "verus";
    inherit version src;

    cargoLock = ./source/Cargo.lock;
    cargoToml = ./source/Cargo.toml;

    nativeBuildInputs = [
      vargo
      makeWrapper
    ] ++ lib.optionals stdenv.isDarwin [
      darwin.autoSignDarwinBinariesHook
    ];

    buildInputs = [
      zlib
    ];

    postUnpack = ''
      cd $sourceRoot/source
      sourceRoot="."
    '';

    # vargo doesn't compose well with the way crane does the deps-only build
    cargoArtifacts = null;
    buildPhaseCargoCommand = "vargo build --release";
    doCheck = false;

    # The toolchain is pinned using Rust
    VARGO_TOOLCHAIN = "host";

    VERUS_Z3_PATH = lib.getExe z3;

    RUST_SYSROOT = rust;

    # For rust_verify
    preBuild = ''
      export LD_LIBRARY_PATH="${rust}/lib";
    '' + lib.optionalString stdenv.isDarwin ''
      export DYLD_LIBRARY_PATH="${rust}/lib";
    '';

    installPhase = ''
      runHook preInstall

      mkdir -p $out/bin $out/lib/verus

      pushd target-verus/release
      cp \
        *.{rlib,so,dylib,vir} \
        .vstd-fingerprint \
        verus-root \
        $out/lib/verus
      popd

      pushd target/release
      cp rust_verify $out/bin
      popd

      wrapProgram $out/bin/rust_verify \
        --set VERUS_ROOT $out/lib/verus \
        --set VERUS_Z3_PATH "$VERUS_Z3_PATH"

      runHook postInstall
    '';

    doNotRemoveReferencesToVendorDir = true;
    dontStrip = true;
    dontPatchELF = true;
  };
in
{
  inherit src vargo verus;
}
