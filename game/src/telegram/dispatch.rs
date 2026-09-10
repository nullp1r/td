//! Routes Telegram messages/callbacks into application operations and player-facing presentation.

use tdx::{
  action::callback as action_callback,
  client::{self, Client},
  enums::CallbackQueryPayload,
  prelude::*,
};
use tokio::sync::watch;

use crate::{
  app::{App, Error as AppError},
  fishing::StruggleAction,
  ids::EncounterId,
  timer,
  view::ReelOutcome,
};

use super::{callback::Callback, present, wake_timer};

/// Handles one incoming bot-command message.
pub async fn message(app: &App, client: &Client, message: &types::message) -> anyhow::Result<()> {
  let Some(command) = message.command() else { return Ok(()) };
  if message.chat_id < 0 { group_command(app, client, message, command.name).await } else { private_command(app, client, message, command.name).await }
}

async fn group_command(app: &App, client: &Client, message: &types::message, command: &str) -> anyhow::Result<()> {
  match command {
    "fish" => {
      present::group_fish(client, message.chat_id, &app.group_event(message.chat_id, timer::now_ms()).await?).await?;
    }
    "help" => {
      let Some(user_id) = message.sender_id.user_id() else { return Ok(()) };
      present::group_help_send(client, message, user_id).await?;
    }
    _ => {}
  }
  Ok(())
}

async fn private_command(app: &App, client: &Client, message: &types::message, command: &str) -> anyhow::Result<()> {
  let Some(user_id) = message.sender_id.user_id() else { return Ok(()) };
  match command {
    "start" => {
      present::home(client, message.chat_id, &app.ensure_character(user_id, timer::now_ms()).await?).await?;
    }
    _ => {
      present::help_send(client, message.chat_id).await?;
    }
  }
  Ok(())
}

/// Handles one incoming callback query.
pub async fn callback(app: &App, client: &Client, update: &types::updateNewCallbackQuery, wake: &watch::Sender<u64>) -> anyhow::Result<()> {
  let CallbackQueryPayload::callbackQueryPayloadData(payload) = &update.payload else {
    client.send(&action_callback::ack(update)).await?;
    return Ok(());
  };
  let Some(callback) = Callback::decode(&payload.data) else {
    client.send(&action_callback::ack(update)).await?;
    return Ok(());
  };
  // Group callbacks own their acknowledgement because an ephemeral response may need to fall back to a toast.
  if !matches!(callback, Callback::GroupCast { .. } | Callback::GroupJournal | Callback::GroupRecords | Callback::GroupHelp) {
    client.send(&action_callback::ack(update)).await?;
  }

  match callback {
    Callback::Cast | Callback::Reel { .. } | Callback::CancelFishing | Callback::Pull { .. } | Callback::GiveLine { .. } => {
      fishing_callback(app, client, update, wake, callback).await?;
    }
    Callback::BuyBait { .. }
    | Callback::SelectBait { .. }
    | Callback::SellAll
    | Callback::ForageBait
    | Callback::BuyRod { .. }
    | Callback::EquipRod { .. }
    | Callback::RepairRods
    | Callback::Craft { .. } => economy_callback(app, client, update, callback).await?,
    Callback::ClaimObjective { .. } | Callback::TurnInContract | Callback::EquipTitle { .. } => {
      progression_callback(app, client, update, callback).await?;
    }
    Callback::GroupCast { .. } | Callback::GroupJournal | Callback::GroupRecords | Callback::GroupHelp => group_callback(app, client, update, callback).await?,
    Callback::Home
    | Callback::Inventory
    | Callback::Journal
    | Callback::Locations
    | Callback::Shop
    | Callback::Help
    | Callback::Travel { .. }
    | Callback::Explore
    | Callback::Tasks
    | Callback::Talk
    | Callback::Conditions
    | Callback::Records
    | Callback::Titles
    | Callback::Crafting => panel_callback(app, client, update, callback).await?,
  }
  Ok(())
}

async fn fishing_callback(
  app: &App,
  client: &Client,
  update: &types::updateNewCallbackQuery,
  wake: &watch::Sender<u64>,
  callback: Callback,
) -> anyhow::Result<()> {
  let now = timer::now_ms();
  match callback {
    Callback::Cast => {
      let seed = getrandom::u64().map_err(|error| anyhow::anyhow!("failed to obtain cast entropy: {error}"))?;
      match app.cast(update.sender_user_id, update.chat_id, update.message_id, seed, now).await {
        Ok(()) => {
          wake_timer(wake);
          present::casting(client, update.chat_id, update.message_id, &app.character(update.sender_user_id, now).await?).await?;
        }
        Err(error) => present_app_error(client, update, error).await?,
      }
    }
    Callback::Reel { encounter_id, step } => {
      match app.reel(update.sender_user_id, encounter_id, step, now).await {
        Ok(outcome) => present_reel_outcome(app, client, outcome).await?,
        Err(error) => present_app_error(client, update, error).await?,
      }
      wake_timer(wake);
    }
    Callback::CancelFishing => match app.cancel_fishing(update.sender_user_id, now).await {
      Ok(character) => {
        wake_timer(wake);
        present::home_edit(client, update.chat_id, update.message_id, &character).await?;
      }
      Err(error) => present_app_error(client, update, error).await?,
    },
    Callback::Pull { encounter_id, step } => {
      struggle(app, client, update, wake, encounter_id, step, StruggleAction::Pull).await?;
    }
    Callback::GiveLine { encounter_id, step } => {
      struggle(app, client, update, wake, encounter_id, step, StruggleAction::GiveLine).await?;
    }
    _ => unreachable!("fishing callback group is exhaustive"),
  }
  Ok(())
}

async fn struggle(
  app: &App,
  client: &Client,
  update: &types::updateNewCallbackQuery,
  wake: &watch::Sender<u64>,
  encounter_id: EncounterId,
  step: u32,
  action: StruggleAction,
) -> anyhow::Result<()> {
  let now = timer::now_ms();
  match app.struggle_action(update.sender_user_id, encounter_id, step, action, now).await {
    Ok(outcome) => present_reel_outcome(app, client, outcome).await?,
    Err(error) => present_app_error(client, update, error).await?,
  }
  wake_timer(wake);
  Ok(())
}

async fn economy_callback(app: &App, client: &Client, update: &types::updateNewCallbackQuery, callback: Callback) -> anyhow::Result<()> {
  let now = timer::now_ms();
  match callback {
    Callback::BuyBait { bait_id } => match app.buy_bait(update.sender_user_id, bait_id).await {
      Ok(shop) => present::shop(client, update.chat_id, update.message_id, &shop).await?,
      Err(error) => present_app_error(client, update, error).await?,
    },
    Callback::SelectBait { bait_id } => match app.select_bait(update.sender_user_id, bait_id).await {
      Ok(inventory) => present::inventory(client, update.chat_id, update.message_id, &inventory).await?,
      Err(error) => present_app_error(client, update, error).await?,
    },
    Callback::SellAll => match app.sell_all_catches(update.sender_user_id, now).await {
      Ok(sale) => present::sold(client, update.chat_id, update.message_id, sale, &app.shop(update.sender_user_id).await?).await?,
      Err(error) => present_app_error(client, update, error).await?,
    },
    Callback::ForageBait => match app.forage_bait(update.sender_user_id).await {
      Ok(shop) => present::shop(client, update.chat_id, update.message_id, &shop).await?,
      Err(error) => present_app_error(client, update, error).await?,
    },
    Callback::BuyRod { rod_id } => match app.buy_rod(update.sender_user_id, rod_id, now).await {
      Ok(shop) => present::shop(client, update.chat_id, update.message_id, &shop).await?,
      Err(error) => present_app_error(client, update, error).await?,
    },
    Callback::EquipRod { rod_id } => match app.equip_rod(update.sender_user_id, rod_id).await {
      Ok(inventory) => present::inventory(client, update.chat_id, update.message_id, &inventory).await?,
      Err(error) => present_app_error(client, update, error).await?,
    },
    Callback::RepairRods => match app.repair_rods(update.sender_user_id).await {
      Ok(shop) => present::shop(client, update.chat_id, update.message_id, &shop).await?,
      Err(error) => present_app_error(client, update, error).await?,
    },
    Callback::Craft { recipe_id } => match app.craft(update.sender_user_id, recipe_id, now).await {
      Ok(result) => present::crafted(client, update.chat_id, update.message_id, &result, &app.crafting(update.sender_user_id).await?).await?,
      Err(error) => present_app_error(client, update, error).await?,
    },
    _ => unreachable!("economy callback group is exhaustive"),
  }
  Ok(())
}

async fn progression_callback(app: &App, client: &Client, update: &types::updateNewCallbackQuery, callback: Callback) -> anyhow::Result<()> {
  let now = timer::now_ms();
  match callback {
    Callback::ClaimObjective { objective_id } => match app.claim_objective(update.sender_user_id, objective_id, now).await {
      Ok(reward) => present::reward(client, update.chat_id, update.message_id, &reward, &app.tasks(update.sender_user_id, now).await?).await?,
      Err(error) => present_app_error(client, update, error).await?,
    },
    Callback::TurnInContract => match app.turn_in_contract(update.sender_user_id, now).await {
      Ok(reward) => present::reward(client, update.chat_id, update.message_id, &reward, &app.tasks(update.sender_user_id, now).await?).await?,
      Err(error) => present_app_error(client, update, error).await?,
    },
    Callback::EquipTitle { title_id } => match app.equip_title(update.sender_user_id, title_id).await {
      Ok(titles) => present::titles(client, update.chat_id, update.message_id, &titles).await?,
      Err(error) => present_app_error(client, update, error).await?,
    },
    _ => unreachable!("progression callback group is exhaustive"),
  }
  Ok(())
}

async fn panel_callback(app: &App, client: &Client, update: &types::updateNewCallbackQuery, callback: Callback) -> anyhow::Result<()> {
  let now = timer::now_ms();
  match callback {
    Callback::Home => present::home_edit(client, update.chat_id, update.message_id, &app.ensure_character(update.sender_user_id, now).await?).await?,
    Callback::Inventory => present::inventory(client, update.chat_id, update.message_id, &app.inventory(update.sender_user_id).await?).await?,
    Callback::Journal => present::journal(client, update.chat_id, update.message_id, &app.journal(update.sender_user_id).await?).await?,
    Callback::Locations => present::locations(client, update.chat_id, update.message_id, &app.locations(update.sender_user_id).await?).await?,
    Callback::Shop => present::shop(client, update.chat_id, update.message_id, &app.shop(update.sender_user_id).await?).await?,
    Callback::Help => present::help(client, update.chat_id, update.message_id).await?,
    Callback::Travel { location_id } => match app.travel(update.sender_user_id, location_id, now).await {
      Ok(character) => present::home_edit(client, update.chat_id, update.message_id, &character).await?,
      Err(error) => present_app_error(client, update, error).await?,
    },
    Callback::Explore => match app.explore(update.sender_user_id, now).await {
      Ok(exploration) => present::explored(client, update.chat_id, update.message_id, &exploration).await?,
      Err(error) => present_app_error(client, update, error).await?,
    },
    Callback::Tasks => present::tasks(client, update.chat_id, update.message_id, &app.tasks(update.sender_user_id, now).await?).await?,
    Callback::Talk => match app.npc(update.sender_user_id, now).await {
      Ok(npc) => present::npc(client, update.chat_id, update.message_id, &npc).await?,
      Err(error) => present_app_error(client, update, error).await?,
    },
    Callback::Conditions => present::conditions(client, update.chat_id, update.message_id, &app.conditions(update.sender_user_id, now).await?).await?,
    Callback::Records => present::records(client, update.chat_id, update.message_id, &app.records(update.sender_user_id).await?).await?,
    Callback::Titles => present::titles(client, update.chat_id, update.message_id, &app.titles(update.sender_user_id).await?).await?,
    Callback::Crafting => present::crafting(client, update.chat_id, update.message_id, &app.crafting(update.sender_user_id).await?).await?,
    _ => unreachable!("panel callback group is exhaustive"),
  }
  Ok(())
}

async fn group_callback(app: &App, client: &Client, update: &types::updateNewCallbackQuery, callback: Callback) -> anyhow::Result<()> {
  if matches!(callback, Callback::GroupHelp) {
    finish_ephemeral(client, update, present::group_help(client, update).await, "group help", "Couldn't open group fishing help here.").await?;
    return Ok(());
  }

  let now = timer::now_ms();
  app.ensure_character(update.sender_user_id, now).await?;
  match callback {
    Callback::GroupCast { cycle } => group_cast_callback(app, client, update, cycle, now).await?,
    Callback::GroupJournal => {
      let view = app.journal(update.sender_user_id).await?;
      finish_ephemeral(client, update, present::group_journal(client, update, &view).await, "Journal", "Couldn't open your Journal here.").await?;
    }
    Callback::GroupRecords => {
      let view = app.records(update.sender_user_id).await?;
      finish_ephemeral(client, update, present::group_records(client, update, &view).await, "Records", "Couldn't open your Records here.").await?;
    }
    _ => unreachable!("group callback group is exhaustive"),
  }
  Ok(())
}

async fn group_cast_callback(app: &App, client: &Client, update: &types::updateNewCallbackQuery, cycle: i64, now_ms: i64) -> anyhow::Result<()> {
  let seed = getrandom::u64().map_err(|error| anyhow::anyhow!("failed to obtain group-cast entropy: {error}"))?;
  match app.group_cast(update.sender_user_id, update.chat_id, cycle, seed, now_ms).await {
    Ok(catch) => {
      if let Err(error) = present::group_result(client, update, &catch).await {
        tracing::warn!(?error, chat_id = update.chat_id, user_id = update.sender_user_id, "ephemeral group catch presentation failed");
        let toast = action_callback::toast(
          update,
          format_args!("Caught {} · {:.2} kg · +{} XP", catch.species_name, f64::from(catch.weight_g) / 1_000.0, catch.xp_gained),
        );
        client.send(&toast).await?;
      } else {
        client.send(&action_callback::ack(update)).await?;
      }
      // Public edits are intentionally sparse; personal catch detail stays ephemeral.
      if catch.global_first || catch.event.participants.is_power_of_two() {
        present::group_caught(client, update.chat_id, update.message_id, &catch).await?;
      }
    }
    Err(AppError::GroupEventClaimed) => {
      client.send(&action_callback::toast(update, "You already cast into this shoal.")).await?;
    }
    Err(AppError::GroupEventExpired) => {
      client.send(&action_callback::toast(update, "That shoal moved on. Use /fish for the current one.")).await?;
    }
    Err(error) => {
      tracing::warn!(?error, chat_id = update.chat_id, "group shoal action failed");
      client.send(&action_callback::alert(update, "The shared cast could not be completed.")).await?;
    }
  }
  Ok(())
}

async fn finish_ephemeral(
  client: &Client,
  update: &types::updateNewCallbackQuery,
  result: client::Result<()>,
  panel: &'static str,
  fallback: &'static str,
) -> client::Result<()> {
  match result {
    Ok(()) => {
      client.send(&action_callback::ack(update)).await?;
    }
    Err(error) => {
      tracing::warn!(?error, chat_id = update.chat_id, user_id = update.sender_user_id, panel, "ephemeral group panel failed");
      client.send(&action_callback::toast(update, fallback)).await?;
    }
  }
  Ok(())
}

async fn present_reel_outcome(app: &App, client: &Client, outcome: ReelOutcome) -> anyhow::Result<()> {
  match outcome {
    ReelOutcome::Caught(catch) => present::caught(client, &catch).await?,
    ReelOutcome::Struggle(struggle) => {
      present::struggle(client, &struggle).await?;
      app.mark_presented(struggle.encounter_id, struggle.step, timer::now_ms()).await?;
    }
    ReelOutcome::Relic(relic) => present::relic(client, &relic).await?,
    ReelOutcome::Escaped(escape) => present::escaped(client, &escape).await?,
  }
  Ok(())
}

async fn present_app_error(client: &Client, update: &types::updateNewCallbackQuery, error: AppError) -> client::Result<()> {
  let (text, shop_recovery) = match error {
    AppError::CharacterMissing => ("Use /start first.", false),
    AppError::NoBait => ("You're out of usable bait. Open the shop or select another bait from your inventory.", true),
    AppError::EncounterActive => ("You already have a line in the water. Return to that encounter or cut the active line from Home.", false),
    AppError::StaleEncounter => ("That opportunity has already passed.", false),
    AppError::WrongPlayer => ("That fishing interaction belongs to somebody else.", false),
    AppError::NotEnoughCoins => ("You don't have enough coins. Sell stored catches from Inventory, or choose a cheaper bait.", true),
    AppError::NothingToSell => ("You don't have any stored catches to sell.", false),
    AppError::UnknownLocation => ("That location isn’t available right now. Open Locations and choose another place.", false),
    AppError::LocationLocked => ("You haven't discovered that location yet. Explore the places you already know.", false),
    AppError::NotFishable => ("There is nowhere to cast here. Explore the location instead.", false),
    AppError::UnknownRod => ("That rod isn’t available right now. Open Inventory and choose another rod.", false),
    AppError::RodNotOwned => ("You don't own that rod.", false),
    AppError::RodAlreadyOwned => ("You already own that rod. Equip it from Inventory.", false),
    AppError::BaitStillAvailable => ("You still have usable bait. Select it from Inventory before digging for emergency worms.", false),
    AppError::UnknownObjective => ("That Harbor Board milestone isn’t available anymore. Reopen the Board.", false),
    AppError::ObjectiveIncomplete => ("That milestone is not complete yet.", false),
    AppError::ObjectiveClaimed => ("You already claimed that milestone reward.", false),
    AppError::ContractNotReady => ("You do not have a stored specimen matching the current harbor contract.", false),
    AppError::ContractClaimed => ("You already completed the current harbor contract.", false),
    AppError::NoNpcHere => ("There is nobody to talk to at this location. Mara is usually at Old Harbor.", false),
    AppError::BaitNotForSale => ("That bait is crafted rather than sold. Open Tackle preparation from Home or Inventory.", false),
    AppError::UnknownRecipe => ("That preparation isn’t available anymore. Reopen Tackle Preparation.", false),
    AppError::RecipeNotReady => ("You do not have the specimen required for that preparation.", false),
    AppError::UnknownTitle => ("That title isn’t available anymore. Reopen Titles.", false),
    AppError::TitleLocked => ("That title is still locked. Its requirement is shown on the Titles panel.", false),
    AppError::GroupEventExpired => ("That shared shoal has already moved on. Use /fish to see the current one.", false),
    AppError::GroupEventClaimed => ("You already joined this chat's current shoal.", false),
    AppError::Db(_) => ("The game couldn't complete that action.", false),
  };
  present::app_error(client, update.chat_id, update.message_id, text, shop_recovery).await
}
