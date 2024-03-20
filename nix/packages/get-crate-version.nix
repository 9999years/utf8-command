{
  writeShellApplication,
  cargo,
  jq,
}:
writeShellApplication {
  name = "get-crate-version";

  runtimeInputs = [
    cargo
    jq
  ];

  text = ''
    # Gets the version of `utf8-command` in `Cargo.toml` using
    # `cargo metadata` and `jq`.

    VERSION=$(cargo metadata --format-version 1 \
        | jq -r '.packages[] | select(.name == "utf8-command") | .version')

    echo "Version in \`Cargo.toml\` is $VERSION" 1>&2

    if [[ -z "$VERSION" ]]; then
        echo "I wasn't able to determine the version in \`Cargo.toml\` with \`cargo metadata\`"
        exit 1
    fi

    echo "$VERSION"
  '';
}
