#! /bin/bash

set -e
cd $(dirname $0)
cargo build --release

rm "README.md"
exec &> "README.md"

echo '# GeckoPanda CLI app
```'
target/release/geckopanda -h
echo '```'

echo '
## Build from source
Create your Google Cloud `oauth2` [client secret](
https://console.cloud.google.com/apis/credentials) and download the client
secret file to `geckopandacli/secrets/client_secret.json`. Then proceed to
`cargo build --release` as usual.'

exec &> /dev/tty

echo success

