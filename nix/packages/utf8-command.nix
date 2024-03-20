{
  system,
  lib,
  stdenv,
  libiconv,
  darwin,
  inputs,
  rustPlatform,
  rust-analyzer,
  cargo-release,
}: let
  inherit (inputs) crane advisory-db;
  craneLib = crane.lib.${system};

  commonArgs' = {
    src = craneLib.cleanCargoSource (craneLib.path ../../.);

    nativeBuildInputs = lib.optionals stdenv.isDarwin [
      # Additional darwin specific inputs can be set here
      (libiconv.override {
        enableStatic = true;
        enableShared = false;
      })
      darwin.apple_sdk.frameworks.CoreServices
    ];
  };

  # Build *just* the cargo dependencies, so we can reuse
  # all of that work (e.g. via cachix) when running in CI
  cargoArtifacts = craneLib.buildDepsOnly commonArgs';

  commonArgs =
    commonArgs'
    // {
      inherit cargoArtifacts;
    };

  checks = {
    utf8-command-tests = craneLib.cargoNextest (commonArgs
      // {
        NEXTEST_HIDE_PROGRESS_BAR = "true";
      });
    utf8-command-clippy = craneLib.cargoClippy (commonArgs
      // {
        cargoClippyExtraArgs = "--all-targets -- --deny warnings";
      });
    utf8-command-rustdoc = craneLib.cargoDoc (commonArgs
      // {
        cargoDocExtraArgs = "--document-private-items";
        RUSTDOCFLAGS = "-D warnings";
      });
    utf8-command-fmt = craneLib.cargoFmt commonArgs;
    utf8-command-audit = craneLib.cargoAudit (commonArgs
      // {
        inherit advisory-db;
      });
  };

  devShell = craneLib.devShell {
    inherit checks;

    # Make rust-analyzer work
    RUST_SRC_PATH = rustPlatform.rustLibSrc;

    # Extra development tools (cargo and rustc are included by default).
    packages = [
      rust-analyzer
      cargo-release
    ];
  };
in
  # Build the actual crate itself, reusing the dependency
  # artifacts from above.
  craneLib.buildPackage (commonArgs
    // {
      # Don't run tests; we'll do that in a separate derivation.
      doCheck = false;

      passthru = {
        inherit
          checks
          devShell
          commonArgs
          craneLib
          ;
      };
    })
