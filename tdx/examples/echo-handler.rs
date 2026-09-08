//! An echo handler for an application-owned update loop.
//! For a runnable bot with authorization and shutdown, see the workflow example.

use tdx::client::Result;
use tdx::enums::Update;
use tdx::prelude::*;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  tracing::info!("tdx echo bot example");
  Ok(())
}

/// Dispatches updates smoothly without fighting ownership or struct hierarchies.
pub async fn on_update(client: &Client, update: Update) -> Result<()> {
  match update {
    Update::updateNewMessage(upd) if !upd.message.is_outgoing => {
      let sender_id = upd.message.sender_id();

      if let Some(msg) = upd.message.text() {
        tracing::info!(message_id = upd.message.id, sender_id, text = %msg.text, "new message");

        let text = line("Echo: ") + bold(&msg.text);
        let req = send::reply(&upd.message, text);
        client.track(&req, None, None).await?;
      }
    }
    _ => {}
  }

  Ok(())
}
