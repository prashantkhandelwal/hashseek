use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use hashseek::{LookupOptions, lookup};
use size_format::SizeFormatterBinary;

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Look up BitTorrent metadata without downloading torrent content"
)]
struct Args {
    /// A 40-character BitTorrent v1 infohash or a magnet URI
    infohash: String,

    /// Maximum time to wait for metadata
    #[arg(short, long, default_value_t = 120, value_parser = clap::value_parser!(u64).range(1..))]
    timeout: u64,

    /// Print every file contained in the torrent
    #[arg(short, long)]
    files: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Searching the DHT and trackers for torrent metadata...");

    let metadata = lookup(
        &args.infohash,
        LookupOptions {
            timeout: Duration::from_secs(args.timeout),
            ..Default::default()
        },
    )
    .await?;

    println!();
    println!(
        "Name:             {}",
        metadata.name.as_deref().unwrap_or("<unknown>")
    );
    println!("Infohash:         {}", metadata.info_hash);
    println!(
        "Total size:       {}",
        SizeFormatterBinary::new(metadata.total_size)
    );
    println!("Files:            {}", metadata.files.len());
    println!("Pieces:           {}", metadata.total_pieces);
    println!("Discovered peers: {}", metadata.discovered_peers.len());
    match metadata.seeders {
        Some(seeders) => println!("Seeders:          {seeders}"),
        None => println!("Seeders:          unavailable (DHT does not publish swarm totals)"),
    }

    if args.files {
        println!("\nFiles:");
        for (index, file) in metadata.files.iter().enumerate() {
            println!(
                "  {:>4}. {} ({})",
                index + 1,
                file.path,
                SizeFormatterBinary::new(file.size)
            );
        }
    }

    Ok(())
}
