{
  pkgs ? import <nixpkgs> { },
  installShellFiles,
}:
pkgs.rustPlatform.buildRustPackage rec {
  version = "0.1.0";
  pname = "clockin";
  cargoLock.lockFile = ./Cargo.lock;
  src = pkgs.lib.cleanSource ./.;
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
}
