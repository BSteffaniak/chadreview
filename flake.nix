{
  description = "ChadReview - A GitHub PR review tool built with HyperChad";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Rust toolchain from rust-overlay
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
            "rustfmt"
          ];
        };

        # ===== BASE PACKAGE SETS =====

        # Minimal build tools (base for all shells)
        baseBuildTools = with pkgs; [
          pkg-config
          gnumake
          gcc
          libiconv
          autoconf
          automake
          libtool
          cmake
          ninja
          openssl
          openssl.dev
        ];

        # FLTK-specific packages
        fltkPackages =
          with pkgs;
          [
            fltk
            fontconfig
            freetype
            cairo
            pango
          ]
          ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            # X11 and OpenGL packages (Linux-specific)
            xorg.libX11
            xorg.libXcursor
            xorg.libXfixes
            xorg.libXinerama
            xorg.libXft
            xorg.libXext
            xorg.libXrender
            libGL
            libGLU
            mesa
          ];

        # Egui/wgpu packages (for egui-based apps)
        eguiPackages =
          with pkgs;
          [
            # Cross-platform graphics packages
            vulkan-loader
            vulkan-headers
            vulkan-validation-layers
          ]
          ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            # Linux-specific display and graphics packages
            xorg.libX11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            libGL
            mesa
            wayland
            wayland-protocols
            libxkbcommon
          ];

        # ===== SHELL BUILDERS =====

        # Basic shell for non-GUI components
        mkBasicShell =
          {
            name,
            packages ? [ ],
          }:
          pkgs.mkShell {
            buildInputs = [
              rustToolchain
              pkgs.fish
            ]
            ++ baseBuildTools
            ++ packages;
            shellHook = ''
              echo "🔍 ChadReview ${name} Environment"
              echo "Rust: $(rustc --version)"

              # Only exec fish if we're in an interactive shell (not running a command)
              if [ -z "$IN_NIX_SHELL_FISH" ] && [ -z "$BASH_EXECUTION_STRING" ]; then
                case "$-" in
                  *i*) export IN_NIX_SHELL_FISH=1; exec fish ;;
                esac
              fi
            '';
          };

        # FLTK-based GUI shell
        mkFltkShell =
          {
            name,
            extraPackages ? [ ],
          }:
          pkgs.mkShell {
            buildInputs = [
              rustToolchain
              pkgs.fish
            ]
            ++ baseBuildTools
            ++ fltkPackages
            ++ extraPackages
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.clang ];

            shellHook = ''
              echo "🔍 ChadReview ${name} Environment (FLTK Backend)"
              echo "Rust: $(rustc --version)"

              ${pkgs.lib.optionalString pkgs.stdenv.isLinux ''
                export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath fltkPackages}"
              ''}

              ${pkgs.lib.optionalString pkgs.stdenv.isDarwin ''
                export CC="${pkgs.clang}/bin/clang"
                export CXX="${pkgs.clang}/bin/clang++"
              ''}

              # Only exec fish if we're in an interactive shell (not running a command)
              if [ -z "$IN_NIX_SHELL_FISH" ] && [ -z "$BASH_EXECUTION_STRING" ]; then
                case "$-" in
                  *i*) export IN_NIX_SHELL_FISH=1; exec fish ;;
                esac
              fi
            '';
          };

        # Egui-based GUI shell
        mkEguiShell =
          {
            name,
            extraPackages ? [ ],
          }:
          pkgs.mkShell {
            buildInputs = [
              rustToolchain
              pkgs.fish
            ]
            ++ baseBuildTools
            ++ eguiPackages
            ++ extraPackages
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.clang
              pkgs.darwin.apple_sdk.frameworks.Metal
              pkgs.darwin.apple_sdk.frameworks.MetalKit
            ];

            shellHook = ''
              echo "🔍 ChadReview ${name} Environment (Egui Backend)"
              echo "Rust: $(rustc --version)"

              ${pkgs.lib.optionalString pkgs.stdenv.isLinux ''
                export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath eguiPackages}"
                export VK_ICD_FILENAMES="${pkgs.vulkan-loader}/share/vulkan/icd.d/lvp_icd.x86_64.json"
              ''}

              ${pkgs.lib.optionalString pkgs.stdenv.isDarwin ''
                export CC="${pkgs.clang}/bin/clang"
                export CXX="${pkgs.clang}/bin/clang++"
              ''}

              # Only exec fish if we're in an interactive shell (not running a command)
              if [ -z "$IN_NIX_SHELL_FISH" ] && [ -z "$BASH_EXECUTION_STRING" ]; then
                case "$-" in
                  *i*) export IN_NIX_SHELL_FISH=1; exec fish ;;
                esac
              fi
            '';
          };

      in
      {
        devShells = {
          # ===== MAIN SHELLS =====
          default = pkgs.mkShell {
            # Full development environment with all backends
            buildInputs = [
              rustToolchain
              pkgs.fish
            ]
            ++ baseBuildTools
            ++ pkgs.lib.optionals pkgs.stdenv.isLinux (fltkPackages ++ eguiPackages)
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.clang
            ];

            packages = with pkgs; [
              cargo-watch
              cargo-edit
              cargo-audit
            ];

            shellHook = ''
              echo "🔍 ChadReview Full Development Environment"
              echo "Platform: ${system}"
              echo "Rust: $(rustc --version)"
              echo ""
              echo "Available environments:"
              echo "  Server: .#server"
              echo "  FLTK GUI: .#fltk"
              echo "  Egui OpenGL: .#egui-glow"
              echo "  Egui WGPU: .#egui-wgpu"
              echo "  CI: .#ci"

              ${pkgs.lib.optionalString pkgs.stdenv.isLinux ''
                export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath (fltkPackages ++ eguiPackages)}"
              ''}

              ${pkgs.lib.optionalString pkgs.stdenv.isDarwin ''
                export CC="${pkgs.clang}/bin/clang"
                export CXX="${pkgs.clang}/bin/clang++"
              ''}

              # Only exec fish if we're in an interactive shell (not running a command)
              if [ -z "$IN_NIX_SHELL_FISH" ] && [ -z "$BASH_EXECUTION_STRING" ]; then
                case "$-" in
                  *i*) export IN_NIX_SHELL_FISH=1; exec fish ;;
                esac
              fi
            '';
          };

          # ===== SPECIALIZED SHELLS =====

          ci = mkBasicShell {
            name = "CI";
            packages = [ ];
          };

          server = mkBasicShell {
            name = "Relay Server";
            packages = [ ];
          };

          # ===== FLTK-BASED APPLICATION =====

          fltk = mkFltkShell {
            name = "App";
            extraPackages = with pkgs; [
              cargo-watch
            ];
          };

          # ===== EGUI-BASED APPLICATIONS =====

          egui-glow = mkEguiShell {
            name = "App (OpenGL)";
            extraPackages = with pkgs; [
              cargo-watch
            ];
          };

          egui-wgpu = mkEguiShell {
            name = "App (WGPU)";
            extraPackages =
              with pkgs;
              [
                cargo-watch
                vulkan-loader
              ]
              ++ pkgs.lib.optionals pkgs.stdenv.isLinux [ amdvlk ];
          };
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "chadreview";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          buildInputs = baseBuildTools;
          doCheck = false;
        };
      }
    );
}
