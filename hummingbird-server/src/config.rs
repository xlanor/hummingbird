use camino::Utf8PathBuf;
use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "hummingbird-server", about = "Hummingbird Music Server")]
pub struct Config {
    /// Database URL (e.g., sqlite:///path/to/library.db)
    #[arg(long)]
    pub db: String,

    /// Directory to scan for music files (can be specified multiple times)
    #[arg(long)]
    pub scan_dir: Vec<Utf8PathBuf>,

    /// Address to bind the server to
    #[arg(long, default_value = "0.0.0.0:3000")]
    pub bind: String,
}
