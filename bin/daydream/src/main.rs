//! `daydream`: convert a realm's Btrieve files into a Daydream realm, serve it to the
//! browser, or dump its records as JSON.

use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use daydream_store::realm::convert_dir;
use daydream_store::{Realm, WbtrvDb};

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Convert Btrieve files (`*.DAT`, or `*.VIR` where there is no `.DAT`) into wbtrv32
    /// `.db` files in REALM. The source files are only read.
    Convert {
        /// The directory holding the Btrieve files. Leave it out with `--stock`.
        #[arg(required_unless_present_any = ["stock", "stock_zip"])]
        from: Option<PathBuf>,
        /// The realm directory to write the `.db` files into.
        #[arg(long, short)]
        realm: PathBuf,
        /// Use the stock MajorMUD 1.11p distribution (downloaded once and cached).
        #[arg(long, conflicts_with = "from")]
        stock: bool,
        /// Use this copy of the stock distribution's zip instead of downloading it.
        #[arg(long, conflicts_with = "from")]
        stock_zip: Option<PathBuf>,
    },
    /// Serve REALM to the browser.
    Serve {
        /// The realm directory.
        realm: PathBuf,
        /// The address to listen on. Keep it on localhost: there is no login yet.
        #[arg(long, default_value = "127.0.0.1:7340", env = "DAYDREAM_LISTEN")]
        listen: SocketAddr,
    },
    /// Write a file's records as JSON lines, decoded where Daydream knows the record type.
    Dump {
        /// The realm directory.
        realm: PathBuf,
        /// The file's stem (`WCCITEMS`).
        stem: String,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();
    match Cli::parse().command {
        Command::Convert {
            from,
            realm,
            stock,
            stock_zip,
        } => convert(from, &realm, stock || stock_zip.is_some(), stock_zip),
        Command::Serve { realm, listen } => serve(realm, listen),
        Command::Dump { realm, stem } => dump(&realm, &stem),
    }
}

fn convert(
    from: Option<PathBuf>,
    realm: &std::path::Path,
    stock: bool,
    stock_zip: Option<PathBuf>,
) -> anyhow::Result<()> {
    let from = match (from, stock) {
        (Some(dir), false) => dir,
        (None, true) => daydream_wcc::stock_files::vir_dir(stock_zip.as_deref())
            .context("getting the stock distribution")?,
        _ => bail!("give a directory of Btrieve files, or --stock"),
    };
    let converted = convert_dir(&from, realm)?;
    if converted.is_empty() {
        bail!("no .DAT or .VIR files in {}", from.display());
    }
    for c in &converted {
        let name = c.dest.file_name().unwrap_or_default().to_string_lossy();
        if c.report.skipped > 0 {
            println!(
                "{name}: {} records ({} broke a unique key and were left out, as wbtrv32 does)",
                c.report.records, c.report.skipped
            );
        } else {
            println!("{name}: {} records", c.report.records);
        }
    }
    Ok(())
}

fn serve(realm: PathBuf, listen: SocketAddr) -> anyhow::Result<()> {
    Realm::open(&realm).with_context(|| format!("opening the realm {}", realm.display()))?;
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        let listener = tokio::net::TcpListener::bind(listen)
            .await
            .with_context(|| format!("listening on {listen}"))?;
        tracing::info!("serving {} on http://{listen}/", realm.display());
        axum::serve(listener, daydream_server::router(&realm))
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await?;
        Ok(())
    })
}

fn dump(realm: &std::path::Path, stem: &str) -> anyhow::Result<()> {
    let realm = Realm::open(realm)?;
    let Some(file) = realm.file(stem) else {
        bail!("{} has no {stem}.db", realm.dir().display());
    };
    let db = WbtrvDb::open_read_only(&file.path)?;
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    for (id, data) in db.records()? {
        let value = match file.table {
            Some(t) => serde_json::Value::Object(
                t.to_json(u32::try_from(id)?, &data)
                    .with_context(|| format!("{} record {id}", file.stem))?,
            ),
            None => serde_json::json!({
                "_id": id,
                "_raw_len": data.len(),
                "_raw": daydream_wcc::decode::base64(&data),
            }),
        };
        serde_json::to_writer(&mut out, &value)?;
        out.write_all(b"\n")?;
    }
    out.flush()?;
    Ok(())
}
