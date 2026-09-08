//! Event dispatch and handlers for messages and inline callbacks.

use std::str;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use tdx::enums::{CallbackQueryPayload, Update, User};
use tdx::prelude::*;
use tokio::time::sleep;

use super::data::{RODS, SPOTS, SplitMix64};
use super::state::{GameState, Player, ReelOutcome};
use super::ui;

/// Acquires the game state mutex or propagates a poisoning failure.
fn lock_state(state: &Mutex<GameState>) -> Result<MutexGuard<'_, GameState>> {
  state.lock().map_err(|e| anyhow!("GameState lock poisoned: {e}"))
}

/// Generates a pseudo-random seed from unix timestamp nanos.
fn now_seed() -> u64 {
  let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
  dur.as_secs() ^ u64::from(dur.subsec_nanos())
}

/// Extracts the owner user ID and action payload from callback data formatted as `{owner_id}:{action}`.
fn parse_callback_payload(data: &[u8]) -> Option<(i64, &[u8])> {
  let colon = data.iter().position(|&b| b == b':')?;
  let owner_str = str::from_utf8(&data[..colon]).ok()?;
  let owner_id = owner_str.parse::<i64>().ok()?;
  Some((owner_id, &data[colon + 1..]))
}

/// Parses an unsigned integer ID from a byte slice.
fn parse_id(bytes: &[u8]) -> Option<usize> {
  str::from_utf8(bytes).ok()?.parse().ok()
}

/// Spawns the asynchronous bite lifecycle and bobber animation for a cast line.
fn spawn_cast_cycle(
  client: Client, //.
  user_id: i64,
  chat_id: i64,
  message_id: i64,
  rod_id: usize,
  state: Arc<Mutex<GameState>>,
) {
  tokio::spawn(async move {
    let run = async {
      let rod = RODS.get(rod_id).unwrap_or(&RODS[0]);
      let (min_delay, max_delay) = rod.bite_delay_ms;
      let mut rng = SplitMix64::new(now_seed());
      let delay_range = max_delay.saturating_sub(min_delay).max(1);
      let delay = min_delay + (rng.next_u64() % delay_range);

      // Initial patient wait before a nibble occurs
      sleep(Duration::from_millis(delay)).await;

      // Abort if already reeled in, cancelled, or if mutex was poisoned
      let target = (chat_id, message_id);
      if !lock_state(&state)?.is_cast_active(chat_id, message_id) {
        return Ok(());
      }

      // Live edit: Bobber starts twitching
      if let Err(err) = client.send(&edit::rich(&target, ui::cast_nibble())).await {
        tracing::warn!(error = ?err, chat_id, message_id, "Failed to update bobber nibble animation");
      }

      // Brief tension pause before the full strike
      sleep(Duration::from_millis(1200)).await;

      // Abort if reeled during the pause, or activate bite window
      let (bite_content, bite_kb) = {
        let mut s = lock_state(&state)?;
        if !s.is_cast_active(chat_id, message_id) {
          return Ok(());
        }
        let start = Instant::now();
        let end = start + Duration::from_millis(3500);
        s.set_bite_window(chat_id, message_id, start, end);
        ui::cast_bite(user_id)
      };

      let mut edit = edit::rich(&target, bite_content);
      edit.reply_markup = Some(bite_kb);
      if let Err(err) = client.send(&edit).await {
        tracing::warn!(error = ?err, chat_id, message_id, "Failed to update bite text and keyboard");
      }

      // Wait until reaction window lapses
      sleep(Duration::from_millis(3500)).await;

      // Expire if the player was too slow to react
      let expired = lock_state(&state)?.expire_cast(chat_id, message_id);
      if expired {
        let (miss_content, miss_kb) = ui::cast_missed(user_id, "The fish nibbled the bait clean off and escaped into the deep!");
        let mut edit = edit::rich(&target, miss_content);
        edit.reply_markup = Some(miss_kb);
        if let Err(err) = client.send(&edit).await {
          tracing::warn!(error = ?err, chat_id, message_id, "Failed to send timeout text and keyboard");
        }
      }
      Ok::<(), anyhow::Error>(())
    };

    if let Err(err) = run.await {
      tracing::error!(error = ?err, chat_id, message_id, "Cast cycle aborted");
    }
  });
}

/// Dispatches incoming bot updates to their respective handlers.
pub async fn on_update(
  client: &Client, //.
  update: Update,
  me: &types::user,
  state: &Arc<Mutex<GameState>>,
) -> Result<()> {
  match update {
    Update::updateNewMessage(msg) => on_message(client, &msg.message, me, state).await,
    Update::updateNewCallbackQuery(query) => on_callback(client, &query, state).await,
    _ => Ok(()),
  }
}

/// Handles incoming commands and messages.
async fn on_message(
  client: &Client, //.
  message: &types::message,
  me: &types::user,
  state: &Arc<Mutex<GameState>>,
) -> Result<()> {
  if message.is_outgoing {
    return Ok(());
  }

  let Some(user_id) = message.sender_id.user_id() else {
    return Ok(());
  };

  let Some(cmd) = message.command_for(me.username()) else {
    return Ok(());
  };

  let has_name = lock_state(state)?.player(user_id).is_some_and(|p| !p.name.is_empty());
  if !has_name {
    let User::user(user) = client.send(&fns::getUser { user_id }).await?;
    lock_state(state)?.player_mut(user_id).name = user.display_name().to_string();
  }

  match cmd.name {
    "fish" | "cast" => {
      let (rod_id, spot_id, user_name) = {
        let mut s = lock_state(state)?;
        let p = s.player_mut(user_id);
        (p.rod_id, p.spot_id, p.display_name().to_string())
      };

      let rod = RODS.get(rod_id).unwrap_or(&RODS[0]);
      let spot = SPOTS.get(spot_id).unwrap_or(&SPOTS[0]);

      let (content, markup) = ui::cast_initial(user_id, &user_name, spot.name, rod.name);
      let mut req = send::reply(message, content);
      req.reply_markup = Some(markup);

      let sent = client.track(&req, None, None).await?;
      lock_state(state)?.register_cast(user_id, sent.chat_id, sent.id, rod_id, spot_id);

      spawn_cast_cycle(client.clone(), user_id, sent.chat_id, sent.id, rod_id, state.clone());
    }

    "bag" | "inv" | "inventory" => {
      let (content, markup) = {
        let mut s = lock_state(state)?;
        ui::bag_view(user_id, s.player_mut(user_id))
      };
      let mut req = send::reply(message, content);
      req.reply_markup = Some(markup);
      client.track(&req, None, None).await?;
    }

    "shop" => {
      let (content, markup) = {
        let mut s = lock_state(state)?;
        ui::shop_view(user_id, s.player_mut(user_id))
      };
      let mut req = send::reply(message, content);
      req.reply_markup = Some(markup);
      client.track(&req, None, None).await?;
    }

    "spots" | "travel" => {
      let (content, markup) = {
        let mut s = lock_state(state)?;
        ui::spots_view(user_id, s.player_mut(user_id))
      };
      let mut req = send::reply(message, content);
      req.reply_markup = Some(markup);
      client.track(&req, None, None).await?;
    }

    "brag" => {
      let content = {
        let mut s = lock_state(state)?;
        ui::brag_view(s.player_mut(user_id))
      };
      let req = if message.is_reply() { send::reply(message, content) } else { send::respond(message, content) };
      client.track(&req, None, None).await?;
    }

    "help" | "start" => {
      let content = ui::help_view();
      let req = send::reply(message, content);
      client.track(&req, None, None).await?;
    }

    _ => {}
  }

  Ok(())
}

/// Handles inline keyboard button callbacks.
async fn on_callback(
  client: &Client, //.
  query: &types::updateNewCallbackQuery,
  state: &Arc<Mutex<GameState>>,
) -> Result<()> {
  let CallbackQueryPayload::callbackQueryPayloadData(payload) = &query.payload else {
    return Ok(());
  };

  let Some((owner_id, action)) = parse_callback_payload(&payload.data) else {
    client.send(&callback::ack(query)).await?;
    return Ok(());
  };

  if query.sender_user_id != owner_id {
    client.send(&callback::alert(query, "⚠️ This menu belongs to another angler! Use /fish or /bag to play.")).await?;
    return Ok(());
  }

  match action {
    b"cast:reel" => handle_reel(client, query, owner_id, state).await?,
    b"cast:again" => handle_cast_again(client, query, owner_id, state).await?,
    b"bag:view" => handle_bag_view(client, query, owner_id, state).await?,
    b"bag:sell" => handle_bag_sell(client, query, owner_id, state).await?,
    b"shop:view" => handle_shop_view(client, query, owner_id, state).await?,
    b"spot:view" => handle_spot_view(client, query, owner_id, state).await?,
    data if let Some(rest) = data.strip_prefix(b"shop:buy:") => {
      handle_shop_buy(client, query, owner_id, rest, state).await?;
    }
    data if data.starts_with(b"shop:info:") => {
      client.send(&callback::toast(query, "This rod is already equipped!")).await?;
    }
    data if let Some(rest) = data.strip_prefix(b"spot:go:") => {
      handle_spot_go(client, query, owner_id, rest, state).await?;
    }
    data if data.starts_with(b"spot:info:") => {
      client.send(&callback::toast(query, "You are already fishing at this spot!")).await?;
    }
    data if data.starts_with(b"spot:locked:") => {
      client.send(&callback::alert(query, "You need a higher tier rod to fish here!")).await?;
    }
    _ => {
      client.send(&callback::ack(query)).await?;
    }
  }

  Ok(())
}

/// Handles reeling in when the player reacts to a bite or pulls the line.
async fn handle_reel(
  client: &Client, //.
  query: &types::updateNewCallbackQuery,
  owner_id: i64,
  state: &Arc<Mutex<GameState>>,
) -> Result<()> {
  let now = Instant::now();
  let mut rng = SplitMix64::new(now_seed());
  let (outcome, user_name) = {
    let mut s = lock_state(state)?;
    let outcome = s.reel(owner_id, query.chat_id, query.message_id, now, &mut rng);
    let name = s.player(owner_id).map_or("Angler", Player::display_name).to_string();
    (outcome, name)
  };

  let (toast, (content, kb)) = match outcome {
    ReelOutcome::Hooked { record, lore } => ("🎉 Fish hooked!", ui::cast_success(owner_id, &user_name, &record, lore)),
    ReelOutcome::TooEarly => ("⚠️ Too early! The fish darted away!", ui::cast_missed(owner_id, "You reeled in before the fish took the hook!")),
    ReelOutcome::TooLate => ("💨 Too late! The fish got away!", ui::cast_missed(owner_id, "The fish ate the bait and disappeared into the depths.")),
    ReelOutcome::NotActive => {
      client.send(&callback::toast(query, "This cast is no longer active or belongs to another angler.")).await?;
      return Ok(());
    }
  };

  client.send(&callback::toast(query, toast)).await?;
  let mut edit = edit::rich(query, content);
  edit.reply_markup = Some(kb);
  client.send(&edit).await?;
  Ok(())
}

/// Re-casts the line with the current rod and spot.
async fn handle_cast_again(
  client: &Client, //.
  query: &types::updateNewCallbackQuery,
  owner_id: i64,
  state: &Arc<Mutex<GameState>>,
) -> Result<()> {
  client.send(&callback::toast(query, "Casting line...")).await?;

  let (rod_id, spot_id, user_name) = {
    let mut s = lock_state(state)?;
    let p = s.player_mut(owner_id);
    (p.rod_id, p.spot_id, p.display_name().to_string())
  };

  let rod = RODS.get(rod_id).unwrap_or(&RODS[0]);
  let spot = SPOTS.get(spot_id).unwrap_or(&SPOTS[0]);

  let (content, markup) = ui::cast_initial(owner_id, &user_name, spot.name, rod.name);
  let mut edit = edit::rich(query, content);
  edit.reply_markup = Some(markup);
  client.send(&edit).await?;

  lock_state(state)?.register_cast(owner_id, query.chat_id, query.message_id, rod_id, spot_id);
  spawn_cast_cycle(client.clone(), owner_id, query.chat_id, query.message_id, rod_id, state.clone());
  Ok(())
}

/// Displays the player's inventory bag and balance.
async fn handle_bag_view(
  client: &Client, //.
  query: &types::updateNewCallbackQuery,
  owner_id: i64,
  state: &Arc<Mutex<GameState>>,
) -> Result<()> {
  let (content, markup) = {
    let mut s = lock_state(state)?;
    ui::bag_view(owner_id, s.player_mut(owner_id))
  };
  let mut edit = edit::rich(query, content);
  edit.reply_markup = Some(markup);
  client.send(&edit).await?;
  client.send(&callback::ack(query)).await?;
  Ok(())
}

/// Sells all catches in the player's bag and displays the updated balance.
async fn handle_bag_sell(
  client: &Client, //.
  query: &types::updateNewCallbackQuery,
  owner_id: i64,
  state: &Arc<Mutex<GameState>>,
) -> Result<()> {
  let (count, earned, (content, markup)) = {
    let mut s = lock_state(state)?;
    let (count, earned) = s.sell_all(owner_id);
    let views = ui::bag_view(owner_id, s.player_mut(owner_id));
    (count, earned, views)
  };

  if count == 0 {
    client.send(&callback::toast(query, "Your bag is already empty!")).await?;
  } else {
    client.send(&callback::alert(query, format_args!("Sold {count} catches for {earned} 🪙!"))).await?;
    let mut edit = edit::rich(query, content);
    edit.reply_markup = Some(markup);
    client.send(&edit).await?;
  }
  Ok(())
}

/// Displays the rod shop with purchase and equip options.
async fn handle_shop_view(
  client: &Client, //.
  query: &types::updateNewCallbackQuery,
  owner_id: i64,
  state: &Arc<Mutex<GameState>>,
) -> Result<()> {
  let (content, markup) = {
    let mut s = lock_state(state)?;
    ui::shop_view(owner_id, s.player_mut(owner_id))
  };
  let mut edit = edit::rich(query, content);
  edit.reply_markup = Some(markup);
  client.send(&edit).await?;
  client.send(&callback::ack(query)).await?;
  Ok(())
}

/// Handles purchasing or equipping a rod from the tackle shop.
async fn handle_shop_buy(
  client: &Client, //.
  query: &types::updateNewCallbackQuery,
  owner_id: i64,
  rest: &[u8],
  state: &Arc<Mutex<GameState>>,
) -> Result<()> {
  let Some(rod_id) = parse_id(rest) else {
    client.send(&callback::alert(query, "Invalid rod selection")).await?;
    return Ok(());
  };

  let (res, views) = {
    let mut s = lock_state(state)?;
    let res = s.equip_or_buy_rod(owner_id, rod_id);
    let views = res.is_ok().then(|| ui::shop_view(owner_id, s.player_mut(owner_id)));
    (res, views)
  };

  match (res, views) {
    (Ok(msg), Some((content, markup))) => {
      client.send(&callback::toast(query, msg)).await?;
      let mut edit = edit::rich(query, content);
      edit.reply_markup = Some(markup);
      client.send(&edit).await?;
    }
    (Err(err), _) => {
      client.send(&callback::alert(query, err)).await?;
    }
    _ => {}
  }
  Ok(())
}

/// Displays available fishing spots and requirements.
async fn handle_spot_view(
  client: &Client, //.
  query: &types::updateNewCallbackQuery,
  owner_id: i64,
  state: &Arc<Mutex<GameState>>,
) -> Result<()> {
  let (content, markup) = {
    let mut s = lock_state(state)?;
    ui::spots_view(owner_id, s.player_mut(owner_id))
  };
  let mut edit = edit::rich(query, content);
  edit.reply_markup = Some(markup);
  client.send(&edit).await?;
  client.send(&callback::ack(query)).await?;
  Ok(())
}

/// Handles traveling to a new fishing spot.
async fn handle_spot_go(
  client: &Client, //.
  query: &types::updateNewCallbackQuery,
  owner_id: i64,
  rest: &[u8],
  state: &Arc<Mutex<GameState>>,
) -> Result<()> {
  let Some(spot_id) = parse_id(rest) else {
    client.send(&callback::alert(query, "Invalid spot selection")).await?;
    return Ok(());
  };

  let (res, views) = {
    let mut s = lock_state(state)?;
    let res = s.travel_to_spot(owner_id, spot_id);
    let views = res.is_ok().then(|| ui::spots_view(owner_id, s.player_mut(owner_id)));
    (res, views)
  };

  match (res, views) {
    (Ok(msg), Some((content, markup))) => {
      client.send(&callback::toast(query, msg)).await?;
      let mut edit = edit::rich(query, content);
      edit.reply_markup = Some(markup);
      client.send(&edit).await?;
    }
    (Err(err), _) => {
      client.send(&callback::alert(query, err)).await?;
    }
    _ => {}
  }
  Ok(())
}
