{utf8-command}: let
  inherit
    (utf8-command)
    craneLib
    commonArgs
    ;
in
  craneLib.cargoDoc (commonArgs
    // {
      # The default `cargoDocExtraArgs` is `--no-deps`.
      cargoDocExtraArgs = "--all-features";
      RUSTDOCFLAGS = "-D warnings";
    })
