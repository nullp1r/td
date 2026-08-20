#![expect(dead_code, reason = "it's ok")]

use std::future::Future;

use td_client::{ClientHandle, Error as ClientError};
use td_types::{enums, fns, types};

pub trait ClientHandleExt {
  fn send_text(&self, chat_id: i64, text: impl Into<String>) -> impl Future<Output = Result<enums::Message, ClientError>>;
  fn reply_text(&self, chat_id: i64, message_id: i64, text: impl Into<String>) -> impl Future<Output = Result<enums::Message, ClientError>>;
  fn get_message(&self, chat_id: i64, message_id: i64) -> impl Future<Output = Result<enums::Message, ClientError>>;
  fn get_me(&self) -> impl Future<Output = Result<types::user, ClientError>>;
  fn answer_inline_query(&self, id: i64, results: Vec<enums::InputInlineQueryResult>, cache_time: i32) -> impl Future<Output = Result<(), ClientError>>;
}

impl ClientHandleExt for ClientHandle {
  async fn send_text(&self, chat_id: i64, text: impl Into<String>) -> Result<enums::Message, ClientError> {
    let text = types::formattedText { text: text.into(), ..Default::default() };
    let input_message_content = enums::InputMessageContent::inputMessageText(types::inputMessageText { text, ..Default::default() });
    let req = fns::sendMessage { chat_id, input_message_content, ..Default::default() };
    self.execute(&req).await
  }

  async fn reply_text(&self, chat_id: i64, message_id: i64, text: impl Into<String>) -> Result<enums::Message, ClientError> {
    let reply_to = Some(enums::InputMessageReplyTo::inputMessageReplyToMessage(types::inputMessageReplyToMessage { message_id, ..Default::default() }));
    let text = types::formattedText { text: text.into(), ..Default::default() };
    let input_message_content = enums::InputMessageContent::inputMessageText(types::inputMessageText { text, ..Default::default() });
    let req = fns::sendMessage { chat_id, reply_to, input_message_content, ..Default::default() };
    self.execute(&req).await
  }

  async fn get_message(&self, chat_id: i64, message_id: i64) -> Result<enums::Message, ClientError> {
    let req = fns::getMessage { chat_id, message_id };
    self.execute(&req).await
  }

  async fn get_me(&self) -> Result<types::user, ClientError> {
    let res = self.execute(&fns::getMe {}).await?;
    let enums::User::user(user) = res;
    Ok(user)
  }

  async fn answer_inline_query(&self, inline_query_id: i64, results: Vec<enums::InputInlineQueryResult>, cache_time: i32) -> Result<(), ClientError> {
    let req = fns::answerInlineQuery { inline_query_id, results, cache_time, ..Default::default() };
    self.execute(&req).await.map(|_| ())
  }
}
