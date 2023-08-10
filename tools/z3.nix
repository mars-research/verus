{ lib, fetchFromGitHub, z3 }:

z3.overrideAttrs (old: rec {
  version = "4.12.2";
  src = fetchFromGitHub {
    owner = "Z3Prover";
    repo = "z3";
    rev = "z3-${version}";
    hash = "sha256-DTgpKEG/LtCGZDnicYvbxG//JMLv25VHn/NaF307JYA=";
  };
})
