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

Add dependencies:

```toml
[dependencies]
hashseek = { version = "0.1.1", default-features = false }
anyhow = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The following example looks up a torrent, prints its summary, lists every file,
and shows the peers encountered while retrieving the metadata:

```rust,no_run
use std::time::Duration;

use hashseek::{LookupOptions, lookup};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = LookupOptions {
        timeout: Duration::from_secs(60),
        ..LookupOptions::default()
    };

    let torrent = lookup(
        "310ce0725f032b2cb78cfee4887299c41b073d20",
        options,
    )
    .await?;

    println!(
        "Name:             {}",
        torrent.name.as_deref().unwrap_or("<unknown>")
    );
    println!("Infohash:         {}", torrent.info_hash);
    println!("Total size:       {} bytes", torrent.total_size);
    println!("Files:            {}", torrent.files.len());
    println!("Pieces:           {}", torrent.total_pieces);
    println!("Discovered peers: {}", torrent.discovered_peers.len());

    match torrent.seeders {
        Some(seeders) => println!("Seeders:          {seeders}"),
        None => println!("Seeders:          unavailable"),
    }

    if !torrent.files.is_empty() {
        println!("\nFiles:");
        for (index, file) in torrent.files.iter().enumerate() {
            println!("  {:>4}. {} ({} bytes)", index + 1, file.path, file.size);
        }
    }

    if !torrent.discovered_peers.is_empty() {
        println!("\nDiscovered peers:");
        for peer in &torrent.discovered_peers {
            println!("  {peer}");
        }
    }

    Ok(())
}
```

`lookup` returns after retrieving metadata and does not download any payload
files. `TorrentInfo::seeders` is currently `None` because DHT peer discovery
does not provide swarm totals. Seeder counts require tracker scrape support and
would only represent the trackers that respond.
