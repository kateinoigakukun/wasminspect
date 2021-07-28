let
  oxalica_rust_overlay = import (builtins.fetchTarball https://github.com/oxalica/rust-overlay/archive/f2865d93afeab55ba158717dda47ef38dec331f7.tar.gz);
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
