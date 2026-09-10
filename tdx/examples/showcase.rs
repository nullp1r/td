//! A comprehensive showcase of `tdx` patterns and capabilities.
//!
//! Demonstrates single-buffer formatted text composition, array-friendly
//! keyboards, flexible targeting and replies, uniform content constructors,
//! entity-aware command parsing, and direct generated `TDLib` requests.

use anyhow::{Context as _, Result};
use tdx::client;
use tdx::enums::{CallbackQueryPayload, MessageSender, Update, User};
use tdx::prelude::*;

const COMMANDS: [[&str; 2]; 7] = [
  ["help", "Formatting tour and inline buttons"],
  ["info", "Message and sender inspection"],
  ["quote", "Quote the message you reply to"],
  ["video", "Send a video with live status edits"],
  ["document", "Send and caption a document"],
  ["say", "Reply with command arguments"],
  ["album", "Send a media album"],
];

fn main() {}

/// Registers bot commands and fetches the current authenticated user.
pub async fn configure(client: &Client) -> Result<types::user> {
  let commands = COMMANDS.iter().map(|&[cmd, desc]| {
    let [command, description] = [cmd, desc].map(Into::into);
    types::botCommand { command, description, ..Default::default() }
  });

  let req = fns::setCommands { commands: commands.collect(), ..Default::default() };
  client.send(&req).await?;

  let User::user(me) = client.send(&fns::getMe {}).await?;

  if let Some(username) = me.username() {
    println!("Ready as @{username}");
  }

  Ok(me)
}

/// Dispatches incoming updates. The application controls event polling and concurrency.
pub async fn on_update(client: &Client, update: &Update, me: &types::user) -> Result<()> {
  let source = match update {
    Update::updateNewCallbackQuery(query) => return on_callback(client, query).await,
    Update::updateNewMessage(update) if !update.message.is_outgoing => &update.message,
    _ => return Ok(()),
  };

  // Entity-aware command parsing avoids false positives inside code blocks or quotes
  let Some(cmd) = source.command_for(me.username()) else { return Ok(()) };

  match cmd.name {
    "help" => show_help(client, source).await?,
    "info" => show_info(client, source).await?,
    "quote" => quote_reply(client, source).await?,
    "video" => {
      send_video(client, source).await?;
    }
    "document" => {
      send_document(client, source).await?;
    }
    "say" => {
      let response = line(("You said: ", code(cmd.args)));
      let req = send::reply(source, response);
      client.track(&req, None, None).await?;
    }
    "album" => {
      send_album(client, source).await?;
    }
    _ => {}
  }

  Ok(())
}

/// Demonstrates single-buffer multi-argument formatting and array-friendly keyboards.
async fn show_help(client: &Client, source: &types::message) -> Result<()> {
  let mut text = lines((bold("tdx Feature Tour"), "", "Available commands:"));

  for &[cmd, desc] in &COMMANDS {
    text.push(("\n", format_args!("/{cmd} — {desc}")));
  }

  text.push((
    "\n\n",
    link("TDLib", "https://core.telegram.org/tdlib"),
    " supplies the generated API.\n",
    bold("Formatting can include "),
    italic("nested entities"),
    " directly without macros.\n",
    italic(underline("Styles compose naturally without intermediate text buffers.")),
  ));

  // Keyboards accept fixed-size arrays directly — zero `vec![vec![...]]` boilerplate:
  let keyboard = markup::inline([
    [markup::url("Documentation", "https://core.telegram.org/tdlib"), markup::callback("About", b"about")],
    [markup::callback("Hide buttons", b"hide"), markup::callback("Delete menu", b"delete")],
  ]);

  let mut req = send::respond(source, text);
  req.reply_markup = Some(keyboard);

  client.track(&req, None, None).await?;
  Ok(())
}

/// Demonstrates message inspection and direct sender extraction.
async fn show_info(client: &Client, source: &types::message) -> Result<()> {
  let req = fns::getChat { chat_id: source.chat_id };
  let enums::Chat::chat(chat) = client.send(&req).await?;

  let mut text = lines((bold(chat.title), "", ("Chat ID: ", code(source.chat_id)), ("Message ID: ", code(source.id))));

  if let MessageSender::messageSenderUser(sender) = &source.sender_id {
    let req = fns::getUser { user_id: sender.user_id };
    let User::user(user) = client.send(&req).await?;

    text.push(("\nSender: ", mention(&*user.first_name, user.id)));
    if let Some(username) = user.username() {
      text.push(format_args!(" (@{username})"));
    }
  }

  if let Some(original) = source.text() {
    for entity in &original.entities {
      if let enums::TextEntityType::textEntityTypeTextUrl(url) = &entity.r#type {
        text.push(("\n\n", italic("Found URL: "), code(&url.url)));
      }
    }
  }

  let req = send::reply(source, text);
  client.track(&req, None, None).await?;
  Ok(())
}

/// Quotes a replied-to message, preserving all entities without lossy whitelisting.
pub async fn quote_reply(client: &Client, source: &types::message) -> Result<()> {
  if !source.is_reply() {
    anyhow::bail!("This command must be sent in reply to another message");
  }

  let req = fns::getRepliedMessage { chat_id: source.chat_id, message_id: source.id };
  let enums::Message::message(original) = client.send(&req).await?;
  let text = original.text().context("The replied message has no text or caption")?;

  // Retain all original entities via send::reply_quote
  let quote = reply::quote(text.clone(), 0);
  let req = send::reply_quote(source, quote, "Quoted with original entities:".text());

  client.track(&req, None, None).await?;
  Ok(())
}

/// Sends media with harmonized caption constructors, live edits, and cleanup.
pub async fn send_video(client: &Client, source: &types::message) -> Result<types::message> {
  let status_req = send::reply(source, "Preparing video…".text());
  let status = client.track(&status_req, None, None).await?;

  let thumbnail = file::thumbnail("preview.jpg");

  // Standard Option<formattedText> caption constructor
  let caption = line((bold("A short clip"), " — with a thumbnail."));
  let video = content::video(file::local("clip.mp4"), Some(thumbnail), 12, [1280, 720], Some(caption.into()));

  // Edit message using &types::message as target
  let edit_req = edit::text(&status, line(("Uploading ", code("clip.mp4"), "…")));
  client.send(&edit_req).await?;

  let req = send::respond(source, video);
  let sent = client.track(&req, None, None).await?;

  // Symmetric delete accepting &types::message or coordinate tuples
  let req = delete::message(&status, true);
  client.send(&req).await?;
  Ok(sent)
}

/// Sends a document and modifies its caption via direct response.
pub async fn send_document(client: &Client, source: &types::message) -> Result<types::message> {
  let doc = content::document(file::local("notes.pdf"), None, None);
  let req = send::respond(source, doc);
  let sent = client.track(&req, None, None).await?;

  let new_caption = line(("The ", bold("reference notes"), ", updated."));
  let edit_req = edit::caption(&sent, new_caption);
  let enums::Message::message(updated) = client.send(&edit_req).await?;

  Ok(updated)
}

/// Sends a media album using uniform constructors.
pub async fn send_album(client: &Client, source: &types::message) -> Result<Vec<client::Result<types::message>>> {
  let photo = content::photo(file::local("photo.jpg"), [2048, 2048], Some(line(bold("A photo in album")).into()));
  let video = content::video(file::local("clip.mp4"), None, 12, [1280, 720], None);

  let req = send::album(source.chat_id, [photo.into(), video.into()]);
  let results = client.track_all(&req, None, None).await?;
  Ok(results)
}

/// Handles inline callbacks with symmetric targeting and clean answers.
pub async fn on_callback(client: &Client, query: &types::updateNewCallbackQuery) -> Result<()> {
  let is_about = matches!(&query.payload, CallbackQueryPayload::callbackQueryPayloadData(d) if d.data == b"about");

  let text = if is_about { "This menu demonstrates tdx single-buffer formatting and arrays." } else { "" };
  let answer = callback::answer(query, text, is_about);

  client.send(&answer).await?;

  let CallbackQueryPayload::callbackQueryPayloadData(payload) = &query.payload else { return Ok(()) };

  match &*payload.data {
    b"hide" => {
      let req = edit::markup(query, None);
      client.send(&req).await?;
    }
    b"delete" => {
      let req = delete::message(query, true);
      client.send(&req).await?;
    }
    _ => {}
  }

  Ok(())
}

/// Downloads a file with progress reporting via `client::download`.
pub async fn download_file(client: &Client, file_id: i32) -> Result<types::file> {
  let req = transfer::download(file_id, 1);

  let mut report = |_, progress: client::Progress| {
    println!("Downloaded {} / {} bytes", progress.current, progress.total);
  };

  Ok(client.download(&req, None, Some(&mut report)).await?)
}
