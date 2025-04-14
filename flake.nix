{
  description = "Clockin";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

      in
      {
        packages =
          let
            derivation =
              {
                pkgs,
                installShellFiles,
              }:
              pkgs.rustPlatform.buildRustPackage {
                version = "0.1.0";
                pname = "clockin";
                cargoLock.lockFile = ./Cargo.lock;
                src = pkgs.lib.cleanSourceWith {
                  filter =
                    name: type:
                    let
                      baseName = baseNameOf (toString name);
                    in
                    !(baseName == "flake.nix" || baseName == "target");
                  src = pkgs.lib.cleanSource ./.;
                };
                nativeBuildInputs = [
                  installShellFiles
                ];
                postInstall = ''
                  find_output() {
                    filename=$1
                    find -type f -path "*/out/*" -name "$filename" | head -n1
                  }
                  installShellCompletion --zsh `find_output _clockin`
                  installShellCompletion --bash `find_output clockin.bash`
                  installShellCompletion --fish `find_output clockin.fish`
                '';
              };
            package = pkgs.callPackage derivation { };
          in
          {
            default = package;
            docker = pkgs.dockerTools.buildLayeredImage {
              name = "clockin";
              tag = "latest";
              contents = with pkgs; [
                bash
                nano
                neovim
              ];
              config = {
                Entrypoint = [ "${package}/bin/clockin" ];
              };
            };
          };
      }
    );
}
