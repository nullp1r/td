//! Drives durable fishing timers; SQLite remains authoritative across sleeps and restarts.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::{
  sync::{mpsc, watch},
  time::sleep,
};
use tracing::Instrument as _;

use crate::{app::App, view::TimerOutcome};

pub async fn run(app: App, output: mpsc::Sender<TimerOutcome>, mut wake: watch::Receiver<u64>) {
  // The database is authoritative. `wake` only invalidates a sleep when a newly inserted timer
  // may now be the earliest one; losing a wake never loses the durable timer itself.
  loop {
    let next = match app.next_timer_due_at().await {
      Ok(next) => next,
      Err(error) => {
        tracing::error!(?error, "failed to query next timer");
        sleep(Duration::from_secs(1)).await;
        continue;
      }
    };

    let Some(next_due_at) = next else {
      if wake.changed().await.is_err() {
        return;
      }
      continue;
    };

    let now = now_ms();
    if next_due_at > now {
      let sleep_fut = sleep(Duration::from_millis((next_due_at - now) as u64));
      tokio::pin!(sleep_fut);
      tokio::select! {
        () = &mut sleep_fut => {}
        changed = wake.changed() => {
          if changed.is_err() { return; }
          continue;
        }
      }
    }

    // Drain a bounded batch so a backlog cannot monopolize the runtime forever.
    let now = now_ms();
    let ids = match app.due_timer_ids(now, 64).await {
      Ok(ids) => ids,
      Err(error) => {
        tracing::error!(?error, "failed to query due timers");
        continue;
      }
    };
    for id in ids {
      let span = tracing::info_span!("timer.fire", timer_id = id);
      let outcome = app.fire_timer(id, now_ms()).instrument(span).await;
      match outcome {
        Ok(outcome) => {
          if output.send(outcome).await.is_err() {
            return;
          }
        }
        Err(error) => tracing::error!(timer_id = id, ?error, "timer transition failed"),
      }
    }
  }
}

pub fn now_ms() -> i64 {
  let Ok(elapsed) = SystemTime::now().duration_since(UNIX_EPOCH) else {
    return 0;
  };
  elapsed.as_millis().min(i64::MAX as u128) as i64
}
