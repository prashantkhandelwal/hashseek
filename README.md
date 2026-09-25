# hashseek

[![Build](https://github.com/prashantkhandelwal/hashseek/actions/workflows/build.yml/badge.svg)](https://github.com/prashantkhandelwal/hashseek/actions/workflows/build.yml)

Retrieve BitTorrent v1 metadata from an infohash or magnet URI without
downloading the torrent's payload.

The lookup uses the BitTorrent DHT and trackers to find peers, then uses the
BitTorrent metadata extension to retrieve the torrent name and file list.

## Install the command

From this project directory:

```console
cargo install --path .
```

After installation:

```console
hashseek 310ce0725f032b2cb78cfee4887299c41b073d20
hashseek 310ce0725f032b2cb78cfee4887299c41b073d20 --files
hashseek "magnet:?xt=urn:btih:310ce0725f032b2cb78cfee4887299c41b073d20"
```

Use `--timeout <SECONDS>` to change the default 120-second lookup timeout.

## Use as a library

Add the local project as a dependency:

```toml
[dependencies]
hashseek = { path = "../hashseek", default-features = false }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Then call `lookup`:

```rust,no_run
use hashseek::{LookupOptions, lookup};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let torrent = lookup(
        "310ce0725f032b2cb78cfee4887299c41b073d20",
        LookupOptions::default(),
    )
    .await?;

    println!("{:?}", torrent.name);
    println!("{} files", torrent.files.len());
    println!("{} discovered peers", torrent.discovered_peers.len());
    Ok(())
}
```

`TorrentInfo::seeders` is currently `None`. DHT peer discovery does not return
swarm totals; obtaining a seeder count requires tracker scrape support and only
represents the trackers that respond.
