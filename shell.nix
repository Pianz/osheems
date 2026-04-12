{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    # Tools needed at build time
    pkg-config
    clang
    lld
  ];

  buildInputs = with pkgs; [
    # Compiler and package manager
    rustc
    cargo

    # Development tools
    rust-analyzer
    rustfmt
    clippy

    # System dependencies
    openssl
    sqlite
    udev      # <--- Requis pour serialport (libudev-sys)
    systemd   # <--- Fournit souvent les liens vers libudev
  ];

  # Environment variables
  shellHook = ''
    # Automated path for Cargo binaries (like 'dx')
    export PATH="$HOME/.cargo/bin:$PATH"

    # Library paths for C dependencies
    export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"

    # Indispensable pour que pkg-config trouve les fichiers .pc de Nix
    export PKG_CONFIG_PATH="${pkgs.udev.dev}/lib/pkgconfig:${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.sqlite.dev}/lib/pkgconfig"

    export RUST_BACKTRACE=1

    echo "--- OSHEEMS Environment Ready (NixOS + Rust) ---"
    echo "English mode active for logs and comments."
    echo "Udev and Serialport dependencies loaded."
  '';
}
