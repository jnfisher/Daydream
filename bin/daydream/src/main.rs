//! `daydream`: convert a realm's Btrieve files into a Daydream realm, serve it to the
//! browser, dump its records as JSON, or move it to and from Nightmare Redux's `.mdb`.

use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

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
    /// Export REALM's world in Nightmare Redux's `.mdb` format, for NMR to import.
    ///
    /// Writing the `.mdb` needs Windows (its built-in Jet engine, as NMR uses). Elsewhere,
    /// write a plan with `--plan` and turn it into the `.mdb` on Windows with
    /// `tools/nmr/write-mdb.ps1`.
    NmrExport {
        /// The realm directory.
        realm: PathBuf,
        /// The file to write: a `.mdb`, or with `--plan` or `--json` that format.
        out: PathBuf,
        /// Write the plan `write-mdb.ps1` reads instead of the `.mdb`.
        #[arg(long, conflicts_with = "json")]
        plan: bool,
        /// Write the tables as JSON instead of the `.mdb`.
        #[arg(long)]
        json: bool,
        /// Keep the running game's state (room cash and floor items, shop stock, monster
        /// kills), which NMR's export zeroes by default.
        #[arg(long)]
        keep_runtime: bool,
        /// Text for the file's Info row.
        #[arg(long, default_value = "")]
        custom: String,
    },
    /// Export REALM for MMUD Explorer, as NMR's Export to MMUD Explorer does: the same
    /// tables and values, and only what NMR's export shows players.
    MmeExport {
        /// The realm directory.
        realm: PathBuf,
        /// The file to write: a `.mdb`, or with `--plan` or `--json` that format.
        out: PathBuf,
        /// Write the plan `write-mdb.ps1` reads instead of the `.mdb`.
        #[arg(long, conflicts_with = "json")]
        plan: bool,
        /// Write the tables as JSON instead of the `.mdb`.
        #[arg(long)]
        json: bool,
        /// Not NMR's "Legit" export: allows a custom name, an update URL and the sysop's
        /// own room exclusions.
        #[arg(long)]
        not_legit: bool,
        /// "Custom" name for the Info row.
        #[arg(long, requires = "not_legit")]
        custom: Option<String>,
        /// "Update URL" for the Info row.
        #[arg(long, requires = "not_legit")]
        update_url: Option<String>,
        /// "Dat File Version" for the Info row (NMR's default when left out).
        #[arg(long)]
        dat_version: Option<String>,
        /// "Disable Kai".
        #[arg(long)]
        disable_kai: bool,
        /// Leave out NMR's default room exclusions (sysop and test rooms).
        #[arg(long)]
        no_default_excludes: bool,
        /// "Exclude rooms": leave these rooms out, `MAP:FROM-TO`. Give it once per range.
        #[arg(long = "exclude", value_parser = parse_numbers, requires = "not_legit")]
        exclude: Vec<daydream_nmr::Range>,
        /// "Hide excluded rooms": keep the excluded rooms, their details blanked, instead
        /// of leaving them out.
        #[arg(long)]
        hide_excluded: bool,
        /// "No rooms": write no rooms at all.
        #[arg(long, requires = "not_legit")]
        no_rooms: bool,
    },
    /// Import a Nightmare Redux `.mdb` export (or a JSON export) into REALM. Stop the realm
    /// first.
    NmrImport {
        /// The realm directory.
        realm: PathBuf,
        /// The `.mdb` (or `.json`) to import.
        file: PathBuf,
        /// Leave records that already exist alone (NMR's "Skip"; the default is "Update").
        #[arg(long)]
        skip_existing: bool,
        /// Report what would change without writing.
        #[arg(long)]
        dry_run: bool,
        /// Import only these tables (`Items,Monsters`); the default is every one.
        #[arg(long, value_delimiter = ',')]
        tables: Option<Vec<String>>,
        /// Import only a range of a table's records: `Items=100-200`, or for rooms
        /// `Rooms=MAP:FROM-TO`. Give it once per table.
        #[arg(long = "range", value_parser = parse_range)]
        ranges: Vec<daydream_nmr::Range>,
        /// Of rooms, import only their visible and hidden items.
        #[arg(long, conflicts_with = "not_items")]
        only_items: bool,
        /// Of rooms, import everything but their visible and hidden items.
        #[arg(long)]
        not_items: bool,
        /// Keep a text block's parts past the last part the file has (NMR deletes them).
        #[arg(long)]
        keep_extra_parts: bool,
        /// Do exactly what NMR's import does, quirks and all, not Daydream's stricter rules.
        #[arg(long)]
        like_nmr: bool,
    },
    /// Delete records from REALM, as NMR's Database Deleter does: every record of a table,
    /// or a range. Stop the realm first.
    Delete {
        /// The realm directory.
        realm: PathBuf,
        /// The NMR table (`Items`, `Rooms`, ...).
        table: String,
        /// Only this range: `100-200`, or for rooms `MAP:FROM-TO`. Without it, every record.
        #[arg(long)]
        range: Option<String>,
        /// Delete every record of the table (needed when there is no `--range`).
        #[arg(long, conflicts_with = "range", required_unless_present = "range")]
        all: bool,
        /// Report what would go without deleting.
        #[arg(long)]
        dry_run: bool,
    },
    /// Write REALM back into the realm's Btrieve `WCC*.DAT` files. Unchanged files are
    /// left alone, edits that keep every key go in place, and files with records added,
    /// deleted or re-keyed are rebuilt. Stop the game first.
    WriteDat {
        /// The realm directory.
        realm: PathBuf,
        /// The folder holding the WCC files REALM was converted from.
        wcc: PathBuf,
        /// Write a complete set of files here instead of over WCC.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Writing over WCC, don't first copy each changed file to
        /// `WCC/daydream-backup-<time>`.
        #[arg(long, conflicts_with = "out")]
        no_backup: bool,
        /// Report what would be written without writing.
        #[arg(long)]
        dry_run: bool,
        /// Write even over WCC files that changed since REALM was converted from them.
        #[arg(long)]
        force: bool,
    },
    /// Merge another user file's users into REALM's, as NMR's Merge Users does: users REALM
    /// lacks are copied byte for byte, users it has are skipped. Stop the realm first.
    MergeUsers {
        /// The realm directory.
        realm: PathBuf,
        /// The users to merge in: a `WCCUSER2.DAT` (or `.db`), or a realm directory.
        from: PathBuf,
        /// Report what would be copied without writing.
        #[arg(long)]
        dry_run: bool,
    },
}

/// `Table=FROM-TO` or `Table=MAP:FROM-TO`.
fn parse_range(s: &str) -> Result<daydream_nmr::Range, String> {
    let (table, rest) = s
        .split_once('=')
        .ok_or_else(|| format!("{s}: expected TABLE=FROM-TO"))?;
    let mut range = parse_numbers(rest)?;
    range.table = table.trim().to_string();
    Ok(range)
}

/// `FROM-TO` or `MAP:FROM-TO`.
fn parse_numbers(s: &str) -> Result<daydream_nmr::Range, String> {
    let (map, span) = match s.split_once(':') {
        Some((m, r)) => (
            Some(
                m.trim()
                    .parse()
                    .map_err(|_| format!("{m}: not a map number"))?,
            ),
            r,
        ),
        None => (None, s),
    };
    let (from, to) = span
        .split_once('-')
        .ok_or_else(|| format!("{span}: expected FROM-TO"))?;
    let num = |x: &str| {
        x.trim()
            .parse::<i64>()
            .map_err(|_| format!("{x}: not a number"))
    };
    Ok(daydream_nmr::Range {
        table: String::new(),
        map,
        from: num(from)?,
        to: num(to)?,
    })
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
        Command::NmrExport {
            realm,
            out,
            plan,
            json,
            keep_runtime,
            custom,
        } => nmr_export(&realm, &out, plan, json, keep_runtime, custom),
        Command::MmeExport {
            realm,
            out,
            plan,
            json,
            not_legit,
            custom,
            update_url,
            dat_version,
            disable_kai,
            no_default_excludes,
            exclude,
            hide_excluded,
            no_rooms,
        } => {
            let mut opts = daydream_nmr::mme::MmeOptions {
                legit: !not_legit,
                disable_kai,
                exclude_default: !no_default_excludes,
                exclude_rooms: !exclude.is_empty() || no_rooms,
                excluded: exclude
                    .iter()
                    .map(|r| {
                        Ok(daydream_nmr::mme::Excluded {
                            map: r.map.ok_or_else(|| {
                                anyhow::anyhow!(
                                    "--exclude {}-{}: give the map, MAP:FROM-TO",
                                    r.from,
                                    r.to
                                )
                            })?,
                            from: r.from,
                            to: r.to,
                        })
                    })
                    .collect::<anyhow::Result<_>>()?,
                hide_excluded,
                no_rooms,
                custom: custom.unwrap_or_default(),
                update_url: update_url.unwrap_or_default(),
                ..Default::default()
            };
            if let Some(v) = dat_version {
                opts.dat_version = v;
            }
            mme_export(&realm, &out, plan, json, opts)
        }
        Command::NmrImport {
            realm,
            file,
            skip_existing,
            dry_run,
            tables,
            ranges,
            only_items,
            not_items,
            keep_extra_parts,
            like_nmr,
        } => {
            let opts = daydream_nmr::ImportOptions {
                skip_existing,
                dry_run,
                like_nmr,
                tables,
                ranges,
                room_parts: if only_items {
                    daydream_nmr::RoomParts::OnlyItems
                } else if not_items {
                    daydream_nmr::RoomParts::NotItems
                } else {
                    daydream_nmr::RoomParts::All
                },
                extra_parts: !keep_extra_parts,
                ..Default::default()
            };
            nmr_import(&realm, &file, opts)
        }
        Command::Delete {
            realm,
            table,
            range,
            all: _,
            dry_run,
        } => delete(&realm, &table, range.as_deref(), dry_run),
        Command::MergeUsers {
            realm,
            from,
            dry_run,
        } => merge_users(&realm, &from, dry_run),
        Command::WriteDat {
            realm,
            wcc,
            out,
            no_backup,
            dry_run,
            force,
        } => write_dat(&realm, &wcc, out, no_backup, dry_run, force),
    }
}

fn write_dat(
    realm: &Path,
    wcc: &Path,
    out: Option<PathBuf>,
    no_backup: bool,
    dry_run: bool,
    force: bool,
) -> anyhow::Result<()> {
    use daydream_store::dat::{Action, WriteOptions};
    let realm = Realm::open(realm)?;
    let backup = (out.is_none() && !no_backup).then(|| {
        let at = jiff::Zoned::now().strftime("%Y%m%d-%H%M%S").to_string();
        wcc.join(format!("daydream-backup-{at}"))
    });
    let options = WriteOptions {
        out,
        backup: backup.clone(),
        dry_run,
        force,
    };
    let written = daydream_store::dat::write_dat(&realm, wcc, &options)?;
    let mut changed = false;
    for w in &written {
        let name = w.dest.file_name().unwrap_or_default().to_string_lossy();
        let what = match w.action {
            Action::Unchanged => "unchanged".to_string(),
            Action::Updated(n) => format!("{n} records updated in place"),
            Action::Rebuilt(n) => format!("rebuilt with {n} records"),
        };
        changed |= w.action != Action::Unchanged;
        println!("{name}: {what}");
    }
    if dry_run {
        println!("(dry run: nothing written)");
    } else if let Some(b) = backup.filter(|b| changed && b.exists()) {
        println!("the files as they were are in {}", b.display());
    }
    Ok(())
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

fn nmr_export(
    realm: &Path,
    out: &Path,
    plan: bool,
    json: bool,
    keep_runtime: bool,
    custom: String,
) -> anyhow::Result<()> {
    let realm = Realm::open(realm)?;
    // VB's Date and Time in the US format NMR's users mostly have
    let now = jiff::Zoned::now();
    let opts = daydream_nmr::ExportOptions {
        zero_runtime: !keep_runtime,
        custom,
        date: now.strftime("%-m/%-d/%Y").to_string(),
        time: now.strftime("%-I:%M:%S %p").to_string(),
        layout: daydream_nmr::layout_of_realm(&realm),
    };
    let data = daydream_nmr::export(&realm, &opts)?;
    if json {
        std::fs::write(out, serde_json::to_vec(&data)?)?;
    } else if plan {
        daydream_nmr::jet::write_plan(&data, out)?;
    } else {
        daydream_nmr::jet::write_mdb(&data, out)?;
    }
    for t in &data.tables {
        println!("{}: {} rows", t.name, t.rows.len());
    }
    Ok(())
}

fn mme_export(
    realm: &Path,
    out: &Path,
    plan: bool,
    json: bool,
    mut opts: daydream_nmr::mme::MmeOptions,
) -> anyhow::Result<()> {
    let realm = Realm::open(realm)?;
    // VB's Date and Time in the US format NMR's users mostly have
    let now = jiff::Zoned::now();
    opts.date = now.strftime("%-m/%-d/%Y").to_string();
    opts.time = now.strftime("%-I:%M:%S %p").to_string();
    let data = daydream_nmr::mme::export(&realm, daydream_nmr::layout_of_realm(&realm), &opts)?;
    if json {
        std::fs::write(out, serde_json::to_vec(&data)?)?;
    } else if plan {
        std::fs::write(out, daydream_nmr::mme::plan(&data))?;
    } else {
        daydream_nmr::jet::write_plan_mdb(&daydream_nmr::mme::plan(&data), out)?;
    }
    for t in &data.tables {
        println!("{}: {} rows", t.name, t.rows.len());
    }
    Ok(())
}

fn nmr_import(
    realm: &Path,
    file: &Path,
    mut opts: daydream_nmr::ImportOptions,
) -> anyhow::Result<()> {
    let realm = Realm::open(realm)?;
    opts.layout = daydream_nmr::layout_of_realm(&realm);
    let dry_run = opts.dry_run;
    let is_json = file
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"));
    let data: daydream_nmr::NmrData = if is_json {
        serde_json::from_slice(&std::fs::read(file)?)?
    } else {
        daydream_nmr::mdb::read(file)?
    };
    let mut errors = 0;
    for r in daydream_nmr::import(&realm, &data, &opts)? {
        println!(
            "{}: {} inserted, {} updated, {} unchanged, {} skipped, {} deleted, {} errors",
            r.table,
            r.inserted,
            r.updated,
            r.unchanged,
            r.skipped,
            r.deleted,
            r.errors.len()
        );
        for e in &r.errors {
            eprintln!("  {e}");
        }
        errors += r.errors.len();
    }
    if dry_run {
        println!("dry run: nothing was written");
    }
    if errors > 0 {
        bail!("{errors} rows could not be imported");
    }
    Ok(())
}

fn delete(realm: &Path, table: &str, range: Option<&str>, dry_run: bool) -> anyhow::Result<()> {
    let realm = Realm::open(realm)?;
    let range = match range {
        Some(r) => {
            let mut range = parse_numbers(r).map_err(anyhow::Error::msg)?;
            range.table = table.to_string();
            Some(range)
        }
        None => None,
    };
    let layout = daydream_nmr::layout_of_realm(&realm);
    let r = daydream_nmr::delete_range(&realm, table, range.as_ref(), layout, dry_run)?;
    if dry_run {
        println!(
            "{}: {} records would be deleted (dry run)",
            r.table, r.deleted
        );
    } else {
        println!("{}: {} records deleted", r.table, r.deleted);
    }
    Ok(())
}

fn merge_users(realm: &Path, from: &Path, dry_run: bool) -> anyhow::Result<()> {
    let users_of = |realm: &Realm| {
        realm
            .files_of(daydream_wcc::tables::Table::Users)
            .next()
            .map(|f| f.path.clone())
    };
    let into_realm = Realm::open(realm)?;
    let into_path = users_of(&into_realm).context("the realm has no user file")?;
    // the source: a realm, a .db, or a Btrieve file converted to a scratch .db first
    let scratch = tempfile::tempdir()?;
    let from_path = if from.is_dir() {
        users_of(&Realm::open(from)?).context("that realm has no user file")?
    } else if from
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("db"))
    {
        from.to_path_buf()
    } else {
        let dest = scratch.path().join("users.db");
        WbtrvDb::convert(from, &dest)?;
        dest
    };
    let source = WbtrvDb::open_read_only(&from_path)?;
    let mut into = WbtrvDb::open(&into_path)?;
    let r = daydream_store::merge::merge(&source, &mut into, dry_run)?;
    println!(
        "{} users {}copied, {} skipped (already in the realm)",
        r.copied,
        if dry_run { "would be " } else { "" },
        r.skipped.len()
    );
    for key in &r.skipped {
        let id = String::from_utf8_lossy(key);
        println!("  skipped {}", id.trim_end_matches('\0'));
    }
    Ok(())
}
