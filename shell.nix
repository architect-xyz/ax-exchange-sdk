let
  pkgs = import <nixpkgs> { };
in
pkgs.mkShell {
  packages = with pkgs; [
    rustup
    cargo
  ];
  nativeBuildInputs = with pkgs; [
    rustc
    cargo
    rustup
  ];
  buildInputs =
    with pkgs;
    [ openssl ]
    ++ lib.optionals stdenv.hostPlatform.isLinux [ glib-networking ]
    ++ lib.optionals stdenv.hostPlatform.isDarwin [ darwin.apple_sdk.frameworks.WebKit ];
  shellHook = ''
    rustup default 1.93
    rustup component add rust-src
    rustup component add clippy
    export PATH=/home/$(whoami)/.cargo/bin:$PATH
  '';
}
