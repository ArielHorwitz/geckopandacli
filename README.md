# GeckoPanda CLI app
```
Manage files backed up to Google Drive.

When uploading files with sensitive info, consider encrypting them first.

Usage: geckopanda <COMMAND>

Commands:
  ls    List existing files
  up    Upload file
  dl    Download file
  rm    Delete file
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Installation
Download the app from the [releases page](
https://github.com/ArielHorwitz/geckopandacli/releases/) and place it in a
directory that is in your PATH (e.g. `/usr/bin`).

## Build from source
Create your Google Cloud `oauth2` [client secret](
https://console.cloud.google.com/apis/credentials) and download the client
secret file to `geckopandacli/secrets/client_secret.json`. Then proceed to
`cargo build --release` as usual.
