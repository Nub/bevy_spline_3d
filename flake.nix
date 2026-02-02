{
  description = "Bevy 3D Spline Editor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Bevy dependencies
        buildInputs = with pkgs; [
          # Audio
          alsa-lib

          # Input
          udev

          # Windowing
          libxkbcommon
          wayland

          # Vulkan
          vulkan-loader
        ] ++ (with pkgs.xorg; [
          libX11
          libXcursor
          libXi
          libXrandr
        ]);

        nativeBuildInputs = with pkgs; [
          pkg-config
          clang
          mold
        ];

      in {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

          packages = [
            rustToolchain
          ];

          shellHook = ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath buildInputs}:$LD_LIBRARY_PATH"
            export RUSTFLAGS="-C linker=clang -C link-arg=-fuse-ld=mold"
          '';
        };
      }
    );
}
