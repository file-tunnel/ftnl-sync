{ pkgs, agentCheck }:
pkgs.mkShell {
  packages = [
    # encrypted env files — env/enc/*.env.enc, see env/README.md
    pkgs.sops
    pkgs.age
    pkgs.python3
    pkgs.just
    agentCheck
  ]
  ++ (with pkgs; [
    actionlint
    cargo
    clippy
    git
    jdk21_headless
    jq
    nixfmt
    nodejs_22
    openssl
    pkg-config
    ripgrep
    rust-analyzer
    rustc
    rustfmt
    shellcheck
    shfmt
    sqlite
  ]);

  LANG = if pkgs.stdenv.hostPlatform.isDarwin then "en_US.UTF-8" else "C.UTF-8";
  LC_ALL = if pkgs.stdenv.hostPlatform.isDarwin then "en_US.UTF-8" else "C.UTF-8";

  shellHook = ''
    export FTNL_DEV_SHELL="sync"
    export XDG_CACHE_HOME="''${XDG_CACHE_HOME:-$PWD/.cache/nix-agent}"
  '';
}
