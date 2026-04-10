{
  description = "Nix packaging for cc-switch";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f:
        nixpkgs.lib.genAttrs systems (system:
          f system (import nixpkgs { inherit system; }));
    in
    {
      packages = forAllSystems (system: pkgs:
        let
          cc-switch = pkgs.callPackage ./nix/package.nix { };
        in
        {
          inherit cc-switch;
          default = cc-switch;
        });

      apps = forAllSystems (system: pkgs:
        let
          cc-switch = self.packages.${system}.cc-switch;
        in
        {
          cc-switch = {
            type = "app";
            program = "${cc-switch}/bin/cc-switch";
          };
          default = self.apps.${system}.cc-switch;
        });
    };
}
