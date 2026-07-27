{ config, pkgs, ... }:

{
  packages = [
    pkgs.cargo-edit
    pkgs.jq
  ];

  languages.rust = {
    enable = true;
    channel = "stable";
  };

  tasks."boxpacker:release" = {
    description = "Validate, tag, and publish a BoxPacker release";
    cwd = config.git.root;
    exec = "bash ${./scripts/release.sh}";
    input.version = "";
    showOutput = true;
  };

  enterTest = ''
    cargo fmt --all --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all-targets --all-features
  '';
}
