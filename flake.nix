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
        pkgsStatic = (import nixpkgs { inherit system; }).pkgsStatic;

      in
      rec {
        packages =
          let
            derivation =
              {
                pkgs,
                installShellFiles,
              }:
              pkgs.rustPlatform.buildRustPackage {
                version = "0.4.0";
                pname = "clockin";
                cargoLock.lockFile = ./Cargo.lock;
                src = pkgs.lib.cleanSourceWith {
                  filter =
                    name: type:
                    let
                      baseName = baseNameOf (toString name);
                    in
                    !(builtins.elem baseName [
                      "flake.nix"
                      "flake.lock"
                    ]);
                  src = pkgs.lib.cleanSource ./.;
                  name = "clockin-src";
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
            packageStatic = pkgsStatic.callPackage derivation { };
          in
          {
            default = package;
            docker = pkgs.dockerTools.buildLayeredImage {
              name = "clockin";
              tag = "latest";
              contents = with pkgsStatic; [
                busybox
              ];
              config = {
                Env = [
                  "SHELL=sh"
                  "EDITOR=vi"
                  "PATH=/bin:${packageStatic}/bin"
                ];
                Entrypoint = [ "${packageStatic}/bin/clockin" ];
              };
            };
          };
        devShells = {
          default = pkgs.mkShell {
            buildInputs = [ pkgs.cargo ];
          };
          try = pkgs.mkShell {
            buildInputs = [
              pkgs.busybox
              packages.default
            ];
          };
        };
      }
    );
}
