use tokio::signal;

use td_client::{Client, Result, Sender};
use td_types::enums::{MessageContent, MessageSender, Update, User};
use td_types::{fns, types};

mod bot;

#[tokio::main(flavor = "current_thread")]
async fn main() -> bot::Result {
  bot::run(run).await
}

async fn run(client: &mut Client) -> bot::Result {
  let sender = client.sender();
  let User::user(types::user { id, first_name, .. }) = sender.send(&fns::getMe {}).await?;
  tracing::info!(user_id = id, %first_name, "signed in; send the bot a text message or press Ctrl-C");

  while let Some(update) = tokio::select! {
    update = client.recv() => update?,
    interrupt = signal::ctrl_c() => {
      interrupt?;
      None
    }
  } {
    on_update(&sender, update).await?;
  }

  Ok(())
}

async fn on_update(sender: &Sender, update: Update) -> Result {
  match update {
    Update::updateConnectionState(update) => tracing::info!(state = ?update.state, "connection state changed"),
    Update::updateNewMessage(update) if !update.message.is_outgoing => {
      let types::message { id, chat_id, sender_id, content, .. } = update.message;
      let sender_id = match sender_id {
        MessageSender::messageSenderChat(sender) => sender.chat_id,
        MessageSender::messageSenderUser(sender) => sender.user_id,
      };
      match content {
        MessageContent::messageText(message) => {
          let text = message.text;
          tracing::info!(message_id = id, chat_id, sender_id, text = %text.text, "new message");
          reply_text(sender, chat_id, id, text).await?;
        }
        _ => tracing::info!(message_id = id, chat_id, sender_id, "new non-text message"),
      }
    }
    _ => {}
  }
  Ok(())
}

async fn reply_text(sender: &Sender, chat_id: i64, message_id: i64, text: types::formattedText) -> Result {
  let reply_to = Some(types::inputMessageReplyToMessage { message_id, ..Default::default() }.into());
  let input_message_content = types::inputMessageText { text, ..Default::default() }.into();
  let request = fns::sendMessage { chat_id, reply_to, input_message_content, ..Default::default() };
  sender.send_message(&request).await?;
  Ok(())
}
