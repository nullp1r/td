use std::future::Future as Fut;

use td_client::Client;
use td_types::enums::{File, FileType, Message, User};
use td_types::enums::{InputFile, InputInlineQueryResult, InputMessageContent, InputMessageReplyTo};
use td_types::{fns, types};

pub(crate) trait ClientExt {
  fn reply_text(&self, cid: i64, mid: i64, text: impl Into<String>) -> impl Fut<Output = td_client::Result<Message>>;
  fn reply_document(&self, cid: i64, mid: i64, document: InputFile, caption: Option<String>) -> impl Fut<Output = td_client::Result<Message>>;
  fn get_message(&self, cid: i64, mid: i64) -> impl Fut<Output = td_client::Result<Message>>;
  fn get_me(&self) -> impl Fut<Output = td_client::Result<types::user>>;
  fn download(&self, fid: i32, priority: i32) -> impl Fut<Output = td_client::Result<types::file>>;
  fn upload(&self, path: impl Into<String>, priority: i32) -> impl Fut<Output = td_client::Result<types::file>>;
  fn answer_inline_query(&self, qid: i64, results: Vec<InputInlineQueryResult>, cache_time: i32) -> impl Fut<Output = td_client::Result<()>>;
}

impl ClientExt for Client {
  async fn reply_text(&self, cid: i64, mid: i64, text: impl Into<String>) -> td_client::Result<Message> {
    reply_message(self, cid, mid, text_content(text)).await
  }

  async fn reply_document(&self, cid: i64, mid: i64, doc: InputFile, caption: Option<String>) -> td_client::Result<Message> {
    reply_message(self, cid, mid, document_content(doc, caption)).await
  }

  async fn get_message(&self, cid: i64, mid: i64) -> td_client::Result<Message> {
    let req = fns::getMessage { chat_id: cid, message_id: mid };
    self.send(&req).await
  }

  async fn get_me(&self) -> td_client::Result<types::user> {
    let res = self.send(&fns::getMe {}).await?;
    let User::user(user) = res;
    Ok(user)
  }

  async fn download(&self, fid: i32, priority: i32) -> td_client::Result<types::file> {
    let req = fns::downloadFile { synchronous: true, priority, file_id: fid, ..Default::default() };
    let res = self.send(&req).await?;
    let File::file(file) = res;
    Ok(file)
  }

  async fn upload(&self, path: impl Into<String>, priority: i32) -> td_client::Result<types::file> {
    let file_type = Some(FileType::fileTypeDocument);
    let file = types::inputFileLocal { path: path.into() }.into();
    let req = fns::preliminaryUploadFile { priority, file, file_type };
    let res = self.send(&req).await?;
    let File::file(file) = res;
    Ok(file)
  }

  async fn answer_inline_query(&self, qid: i64, results: Vec<InputInlineQueryResult>, cache_time: i32) -> td_client::Result<()> {
    let req = fns::answerInlineQuery { inline_query_id: qid, results, cache_time, ..Default::default() };
    self.send(&req).await.map(|_| ())
  }
}

async fn reply_message(client: &Client, cid: i64, mid: i64, msg: InputMessageContent) -> td_client::Result<Message> {
  let reply_to = Some(reply_do(mid));
  let req = fns::sendMessage { reply_to, chat_id: cid, input_message_content: msg, ..Default::default() };
  client.send(&req).await
}

fn reply_do(mid: i64) -> InputMessageReplyTo {
  types::inputMessageReplyToMessage { message_id: mid, ..Default::default() }.into()
}

fn formatted_text(text: impl Into<String>) -> types::formattedText {
  let text = text.into();
  types::formattedText { text, ..Default::default() }
}

fn text_content(text: impl Into<String>) -> InputMessageContent {
  let text = formatted_text(text);
  types::inputMessageText { text, ..Default::default() }.into()
}

fn document_content(document: InputFile, caption: Option<String>) -> InputMessageContent {
  let document = types::inputDocument { document, ..Default::default() };
  let caption = caption.map(formatted_text);
  types::inputMessageDocument { document, caption }.into()
}
