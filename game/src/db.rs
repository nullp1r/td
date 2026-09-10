//! Serialized SQLite access on one dedicated thread.
//!
//! `rusqlite::Connection` stays off Tokio worker threads; every mutation therefore has one
//! ordering point, and application transactions remain ordinary synchronous SQLite transactions.

use std::{
  io,
  path::{Path, PathBuf},
  thread,
};

use rusqlite::Connection;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

type Job = Box<dyn FnOnce(&mut Connection) + Send + 'static>;

const MIGRATIONS: [&str; 4] = [
  include_str!("../migrations/0001_init.sql"),
  include_str!("../migrations/0002_progression.sql"),
  include_str!("../migrations/0003_world_loop.sql"),
  include_str!("../migrations/0004_social_crafting.sql"),
];

#[derive(Clone)]
pub struct Db {
  tx: mpsc::Sender<Job>,
}

#[derive(Debug, Error)]
pub enum Error {
  #[error("database worker stopped")]
  WorkerStopped,
  #[error("database error: {0}")]
  Sql(#[from] rusqlite::Error),
  #[error("failed to start database worker: {0}")]
  WorkerSpawn(#[source] io::Error),
  #[error("invalid negative database schema version {0}")]
  NegativeSchemaVersion(i64),
  #[error("database schema version {0} is newer than this binary")]
  SchemaTooNew(usize),
}

impl Db {
  pub async fn open(path: PathBuf) -> Result<Self, Error> {
    let (tx, mut rx) = mpsc::channel::<Job>(256);
    let (ready_tx, ready_rx) = oneshot::channel();

    thread::Builder::new()
      .name("sqlite".into())
      .spawn(move || {
        let mut connection = match open_connection(&path) {
          Ok(connection) => connection,
          Err(error) => {
            let _ = ready_tx.send(Err(error));
            return;
          }
        };
        let _ = ready_tx.send(Ok(()));
        while let Some(job) = rx.blocking_recv() {
          job(&mut connection);
        }
      })
      .map_err(Error::WorkerSpawn)?;

    ready_rx.await.map_err(|_| Error::WorkerStopped)??;
    Ok(Self { tx })
  }

  /// Runs arbitrary connection work; `T` may itself be a domain-specific `Result`.
  pub async fn job<T, F>(&self, f: F) -> Result<T, Error>
  where
    T: Send + 'static,
    F: FnOnce(&mut Connection) -> T + Send + 'static,
  {
    let (tx, rx) = oneshot::channel();
    let job = Box::new(move |connection: &mut Connection| {
      let _ = tx.send(f(connection));
    });
    self.tx.send(job).await.map_err(|_| Error::WorkerStopped)?;
    rx.await.map_err(|_| Error::WorkerStopped)
  }
}

fn open_connection(path: &Path) -> Result<Connection, Error> {
  let mut connection = Connection::open(path)?;
  connection.pragma_update(None, "foreign_keys", true)?;
  connection.pragma_update(None, "journal_mode", "WAL")?;
  connection.pragma_update(None, "synchronous", "FULL")?;

  let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
  let Ok(version) = usize::try_from(version) else {
    return Err(Error::NegativeSchemaVersion(version));
  };
  let Some(pending) = MIGRATIONS.get(version..) else {
    return Err(Error::SchemaTooNew(version));
  };
  // user_version is the number of already-applied entries in MIGRATIONS.
  for migration in pending {
    let transaction = connection.transaction()?;
    transaction.execute_batch(migration)?;
    transaction.commit()?;
  }
  Ok(connection)
}
