{
  stdenv,
  utf8-command-docs,
}: let
  inherit (utf8-command-docs) version;
in
  stdenv.mkDerivation {
    pname = "utf8-command-docs-tarball";
    inherit version;

    src = utf8-command-docs;

    dontConfigure = true;
    dontBuild = true;

    installPhase = ''
      dir=utf8-command-docs-${version}
      mv share/doc \
        "$dir"

      mkdir $out
      tar --create \
        --file $out/utf8-command-docs-${version}.tar.gz \
        --auto-compress \
        --verbose \
        "$dir"
    '';
  }
