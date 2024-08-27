{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    esp-idf = {
      url = "github:mirrexagon/nixpkgs-esp-dev";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = flakes:
    let
      system = "x86_64-linux";
      pkgs = import flakes.nixpkgs { inherit system; };
      esp-idf = flakes.esp-idf.packages.${system};
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        packages = [
          esp-idf.esp-idf-esp32c3
          pkgs.glibc_multi
        ];
      };
    };
}
