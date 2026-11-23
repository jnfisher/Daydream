//! The connection pool ([`Db`]) and the throwaway databases tests use ([`TestDb`]).

use std::sync::RwLock;
use std::time::Duration;

use sqlx::AnyPool;
use sqlx::Executor;
use sqlx::any::AnyPoolOptions;

use crate::db::Result;
use crate::db::dialect::Dialect;
use crate::db::repo::classes::Classes;
use crate::db::repo::items::Items;
use crate::db::repo::messages::Messages;
use crate::db::repo::monsters::{MonsterStates, Monsters};
use crate::db::repo::races::Races;
use crate::db::repo::rooms::{RoomStates, Rooms};
use crate::db::repo::shops::{ShopStocks, Shops};
use crate::db::repo::spells::Spells;
use crate::db::repo::text_blocks::{Actions, TextBlocks};

/// A database: the pool and the dialect of its URL.
///
/// Clone-cheap handles come from wrapping it in an `Arc`. The pool can be closed and
/// reopened in place ([`Db::close`], [`Db::reconnect`]) so a worker holding the `Db`
/// survives an outage.
#[derive(Debug)]
pub struct Db {
    url: String,
    dialect: Dialect,
    pool: RwLock<AnyPool>,
}

impl Db {
    /// Opens a pool on `url` (`sqlite:` or `postgres:`). A SQLite file is created when
    /// missing and opened in WAL mode with `synchronous=NORMAL` and foreign keys on.
    pub async fn connect(url: &str) -> Result<Self> {
        let dialect = Dialect::from_url(url)?;
        let url = normalise_url(dialect, url);
        let pool = open_pool(dialect, &url).await?;
        Ok(Self {
            url,
            dialect,
            pool: RwLock::new(pool),
        })
    }

    /// Opens the pool and applies the migrations.
    pub async fn connect_and_migrate(url: &str) -> Result<Self> {
        let db = Self::connect(url).await?;
        db.migrate().await?;
        Ok(db)
    }

    /// Applies the backend's migrations (`sqlx::migrate!`), a no-op once applied.
    pub async fn migrate(&self) -> Result<()> {
        self.dialect.migrator().run(&self.pool()).await?;
        Ok(())
    }

    /// The backend.
    pub fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// The connection URL as opened.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The current pool (a cheap handle).
    pub fn pool(&self) -> AnyPool {
        self.pool.read().expect("pool lock").clone()
    }

    /// Closes the pool: every later query fails until [`Db::reconnect`].
    pub async fn close(&self) {
        self.pool().close().await;
    }

    /// Opens a fresh pool on the same URL, replacing the current one.
    pub async fn reconnect(&self) -> Result<()> {
        let pool = open_pool(self.dialect, &self.url).await?;
        let old = std::mem::replace(&mut *self.pool.write().expect("pool lock"), pool);
        if !old.is_closed() {
            old.close().await;
        }
        Ok(())
    }

    /// Rooms content: `struct room` minus its live fields.
    pub fn rooms(&self) -> Rooms<'_> {
        Rooms::new(self)
    }
    /// The live part of rooms.
    pub fn room_state(&self) -> RoomStates<'_> {
        RoomStates::new(self)
    }
    /// Items.
    pub fn items(&self) -> Items<'_> {
        Items::new(self)
    }
    /// Known monsters.
    pub fn monsters(&self) -> Monsters<'_> {
        Monsters::new(self)
    }
    /// The live part of known monsters.
    pub fn monster_state(&self) -> MonsterStates<'_> {
        MonsterStates::new(self)
    }
    /// Spells.
    pub fn spells(&self) -> Spells<'_> {
        Spells::new(self)
    }
    /// Classes.
    pub fn classes(&self) -> Classes<'_> {
        Classes::new(self)
    }
    /// Races.
    pub fn races(&self) -> Races<'_> {
        Races::new(self)
    }
    /// Shops.
    pub fn shops(&self) -> Shops<'_> {
        Shops::new(self)
    }
    /// Live shop stock.
    pub fn shop_stock(&self) -> ShopStocks<'_> {
        ShopStocks::new(self)
    }
    /// Messages (WCCMSG).
    pub fn messages(&self) -> Messages<'_> {
        Messages::new(self)
    }
    /// Text blocks (WCCTEXT).
    pub fn text_blocks(&self) -> TextBlocks<'_> {
        TextBlocks::new(self)
    }
    /// Social actions (WCCACTS).
    pub fn actions(&self) -> Actions<'_> {
        Actions::new(self)
    }
}

/// `sqlite:` URLs get `mode=rwc` so the file is created; a bare `sqlite://` means an
/// in-memory database (one connection, see [`open_pool`]). A Windows path with a drive
/// letter is written `sqlite:C:\...`: in `sqlite://C:\...` the URL parser takes `C:` for
/// a host and port and the driver opens `C/\...`, which does not exist.
fn normalise_url(dialect: Dialect, url: &str) -> String {
    match dialect {
        Dialect::Sqlite => {
            let url = match url.strip_prefix("sqlite://") {
                Some(path) if has_drive_letter(path) => format!("sqlite:{path}"),
                _ => url.to_string(),
            };
            let url = url.as_str();
            if is_memory_sqlite(url) || url.contains("mode=") {
                url.to_string()
            } else if url.contains('?') {
                format!("{url}&mode=rwc")
            } else {
                format!("{url}?mode=rwc")
            }
        }
        Dialect::Postgres => url.to_string(),
    }
}

/// `C:\...` or `C:/...`.
pub(crate) fn has_drive_letter(path: &str) -> bool {
    let b = path.as_bytes();
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/')
}

fn is_memory_sqlite(url: &str) -> bool {
    url == "sqlite://"
        || url == "sqlite:"
        || url.contains(":memory:")
        || url.contains("mode=memory")
}

async fn open_pool(dialect: Dialect, url: &str) -> Result<AnyPool> {
    open_pool_from(AnyPoolOptions::new(), dialect, url).await
}

/// [`open_pool`] from `options`, so tests can shorten the lifetimes it keeps.
///
/// Lifetimes: an in-memory SQLite database lives in its one connection, so that
/// connection is never reaped (no `idle_timeout`, no `max_lifetime`); reaping it would
/// empty the database. A file or PostgreSQL store keeps sqlx's defaults (idle connections
/// closed after 10 minutes, every connection replaced after 30): nothing is lost when a
/// connection goes, and replacing them lets a server-side change (a failover, a rotated
/// password, a restarted PostgreSQL) reach the pool.
async fn open_pool_from(options: AnyPoolOptions, dialect: Dialect, url: &str) -> Result<AnyPool> {
    sqlx::any::install_default_drivers();
    let mut options = options.acquire_timeout(Duration::from_secs(30));
    options = match dialect {
        Dialect::Sqlite => {
            // One connection for an in-memory database (each connection would otherwise
            // see its own empty database); a small pool for a file.
            let max = if is_memory_sqlite(url) {
                options = options.idle_timeout(None).max_lifetime(None);
                1
            } else {
                4
            };
            options.max_connections(max).after_connect(|conn, _meta| {
                Box::pin(async move {
                    conn.execute("PRAGMA journal_mode = WAL").await?;
                    conn.execute("PRAGMA synchronous = NORMAL").await?;
                    conn.execute("PRAGMA foreign_keys = ON").await?;
                    conn.execute("PRAGMA busy_timeout = 10000").await?;
                    Ok(())
                })
            })
        }
        Dialect::Postgres => options.max_connections(8),
    };
    Ok(options.connect(url).await?)
}
