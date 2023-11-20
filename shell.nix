let
  oxalica_rust_overlay = import (builtins.fetchTarball https://github.com/oxalica/rust-overlay/archive/e17bfe3baa0487f0671c9ed0e9057d10987ba7f7.tar.gz);
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
