#! /bin/bash

set -e

cd $(dirname $0)

./make-readme.sh

cargo release --no-publish --execute $1

VERSION=v$(grep '^version' Cargo.toml | head -n1 | cut -d '"' -f2)
echo "Version: $VERSION"
gh release create --verify-tag --notes-from-tag $VERSION target/release/geckopanda
