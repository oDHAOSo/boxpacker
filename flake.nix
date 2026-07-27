{
  description = "BoxPacker rectangular 3D packing CLI";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      cargoManifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      supportedSystems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        rec {
          boxpacker = pkgs.rustPlatform.buildRustPackage {
            pname = "boxpacker";
            version = cargoManifest.package.version;

            src = pkgs.lib.fileset.toSource {
              root = ./.;
              fileset = pkgs.lib.fileset.unions [
                ./Cargo.lock
                ./Cargo.toml
                ./benches
                ./src
                ./tests
              ];
            };
            cargoLock.lockFile = ./Cargo.lock;

            meta = {
              description = "Packs rectangular items into rectangular containers";
              homepage = "https://github.com/oDHAOSo/boxpacker";
              mainProgram = "boxpacker";
              platforms = supportedSystems;
            };
          };

          default = boxpacker;
        }
      );

      apps = forAllSystems (
        system:
        let
          app = {
            type = "app";
            program = "${self.packages.${system}.boxpacker}/bin/boxpacker";
            meta.description = "Run the BoxPacker CLI";
          };
        in
        {
          boxpacker = app;
          default = app;
        }
      );
    };
}
