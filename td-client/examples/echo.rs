//! Runs the example echo bot.

use tokio::signal;

use td_client::types::enums::{MessageContent, MessageSender, Update, User};
use td_client::types::{fns, types};
use td_client::{Client, Result, Session};

mod helpers;

#[tokio::main(flavor = "current_thread")]
async fn main() -> helpers::Result {
  helpers::run(run).await
}

async fn run(session: &mut Session, client: Client) -> helpers::Result {
  let User::user(me) = client.send(&fns::getMe {}).await?;
  let types::user { id, first_name, .. } = me;
  tracing::info!(name = %first_name, id, "signed in; send the bot a text message or press Ctrl-C");

  loop {
    let update = tokio::select! {
      update = session.recv() => update,
      signal = signal::ctrl_c() => break signal?,
    };
    let Some(update) = update else { break };
    on_update(&client, update).await?;
  }

  Ok(())
}

async fn on_update(client: &Client, update: Update) -> Result {
  match update {
    Update::updateConnectionState(upd) => {
      tracing::info!(state = ?upd.state, "connection state changed");
    }
    Update::updateNewMessage(upd) if !upd.message.is_outgoing => {
      let types::message { id, chat_id, sender_id, content, .. } = upd.message;
      let sender_id = match sender_id {
        MessageSender::messageSenderChat(s) => s.chat_id,
        MessageSender::messageSenderUser(s) => s.user_id,
      };
      match content {
        MessageContent::messageText(msg) => {
          tracing::info!(message_id = id, chat_id, sender_id, text = %msg.text.text, "new message");
          reply_text(client, chat_id, id, msg.text).await?;
        }
        _ => tracing::info!(message_id = id, chat_id, sender_id, "new non-text message"),
      }
    }
    _ => {}
  }
  Ok(())
}

async fn reply_text(client: &Client, chat_id: i64, message_id: i64, text: types::formattedText) -> Result<types::message> {
  let reply_to = Some(types::inputMessageReplyToMessage { message_id, ..Default::default() }.into());
  let input_message_content = types::inputMessageText { text, ..Default::default() }.into();
  let request = fns::sendMessage { chat_id, reply_to, input_message_content, ..Default::default() };
  client.track(&request, None, None).await
}
