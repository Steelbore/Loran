# SPDX-License-Identifier: GPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 Mohamed Hammad
{
  description = "loran — agent-native reference manual for Spacecraft Software";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
        pname = "loran";
        version = "0-unstable";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;

        # Build only the loran binary crate (excludes xtask, which is dev-only).
        cargoBuildFlags = [ "-p" "loran" ];
        cargoTestFlags = [ "-p" "loran" ];

        meta = {
          description = "Agent-native reference manual for Spacecraft Software";
          homepage = "https://github.com/Spacecraft-Software/Loran";
          license = pkgs.lib.licenses.gpl3Plus;
          mainProgram = "loran";
        };
      };

      apps.${system}.default = {
        type = "app";
        program = "${self.packages.${system}.default}/bin/loran";
      };
    };
}
