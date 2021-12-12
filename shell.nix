let
  oxalica_rust_overlay = import (builtins.fetchTarball https://github.com/oxalica/rust-overlay/archive/5e677c6fed9077cb9e2d037e2679676895b6e953.tar.gz);
  pkgs = import <nixpkgs> {
    overlays = [ oxalica_rust_overlay ];
  };
  rust_toolchain = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml);
in
  with pkgs;

  pkgs.mkShell rec {
    nativeBuildInputs = lib.optionals stdenv.isDarwin [
      darwin.libiconv
    ] ++ [
      rust_toolchain
      cacert
      curl
    ];
  }
