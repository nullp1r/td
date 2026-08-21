#![expect(dead_code, reason = "it's ok")]

use std::future::Future as Fut;

use td_client::{ClientHandle, Error as ClientError};
use td_types::{enums::*, fns, types};

pub trait ClientExt {
  fn send_text(&self, cid: i64, text: impl Into<String>) -> impl Fut<Output = Result<Message, ClientError>>;
  fn reply_text(&self, cid: i64, mid: i64, text: impl Into<String>) -> impl Fut<Output = Result<Message, ClientError>>;
  fn send_document(&self, cid: i64, path: impl Into<String>, caption: Option<String>) -> impl Fut<Output = Result<Message, ClientError>>;
  fn reply_document(&self, cid: i64, mid: i64, document: InputFile, caption: Option<String>) -> impl Fut<Output = Result<Message, ClientError>>;
  fn get_message(&self, cid: i64, mid: i64) -> impl Fut<Output = Result<Message, ClientError>>;
  fn get_chat(&self, cid: i64) -> impl Fut<Output = Result<types::chat, ClientError>>;
  fn get_me(&self) -> impl Fut<Output = Result<types::user, ClientError>>;
  fn download(&self, fid: i32, priority: i32) -> impl Fut<Output = Result<types::file, ClientError>>;
  fn upload(&self, path: impl Into<String>, priority: i32) -> impl Fut<Output = Result<types::file, ClientError>>;
  fn answer_inline_query(&self, qid: i64, results: Vec<InputInlineQueryResult>, cache_time: i32) -> impl Fut<Output = Result<(), ClientError>>;
}

impl ClientExt for ClientHandle {
  async fn send_text(&self, cid: i64, text: impl Into<String>) -> Result<Message, ClientError> {
    send_message(self, cid, None, text_content(text)).await
  }

  async fn reply_text(&self, cid: i64, mid: i64, text: impl Into<String>) -> Result<Message, ClientError> {
    send_message(self, cid, Some(mid), text_content(text)).await
  }

  async fn send_document(&self, cid: i64, path: impl Into<String>, caption: Option<String>) -> Result<Message, ClientError> {
    let document = InputFile::inputFileLocal(types::inputFileLocal { path: path.into() });
    send_message(self, cid, None, document_content(document, caption)).await
  }

  async fn reply_document(&self, cid: i64, mid: i64, document: InputFile, caption: Option<String>) -> Result<Message, ClientError> {
    send_message(self, cid, Some(mid), document_content(document, caption)).await
  }

  async fn get_message(&self, cid: i64, mid: i64) -> Result<Message, ClientError> {
    let req = fns::getMessage { chat_id: cid, message_id: mid };
    self.execute(&req).await
  }

  async fn get_chat(&self, cid: i64) -> Result<types::chat, ClientError> {
    let res = self.execute(&fns::getChat { chat_id: cid }).await?;
    let Chat::chat(chat) = res;
    Ok(chat)
  }

  async fn get_me(&self) -> Result<types::user, ClientError> {
    let res = self.execute(&fns::getMe {}).await?;
    let User::user(user) = res;
    Ok(user)
  }

  async fn download(&self, fid: i32, priority: i32) -> Result<types::file, ClientError> {
    let req = fns::downloadFile { synchronous: true, priority, file_id: fid, ..Default::default() };
    let res = self.execute(&req).await?;
    let File::file(file) = res;
    Ok(file)
  }

  async fn upload(&self, path: impl Into<String>, priority: i32) -> Result<types::file, ClientError> {
    let input = InputFile::inputFileLocal(types::inputFileLocal { path: path.into() });
    let req = fns::preliminaryUploadFile { priority, file: input, file_type: Some(FileType::fileTypeDocument) };
    let res = self.execute(&req).await?;
    let File::file(file) = res;
    Ok(file)
  }

  async fn answer_inline_query(&self, qid: i64, results: Vec<InputInlineQueryResult>, cache_time: i32) -> Result<(), ClientError> {
    let req = fns::answerInlineQuery { inline_query_id: qid, results, cache_time, ..Default::default() };
    self.execute(&req).await.map(|_| ())
  }
}

async fn send_message(client: &ClientHandle, cid: i64, mid: Option<i64>, msg: InputMessageContent) -> Result<Message, ClientError> {
  let req = fns::sendMessage { chat_id: cid, reply_to: mid.map(reply_do), input_message_content: msg, ..Default::default() };
  client.execute(&req).await
}

fn reply_do(mid: i64) -> InputMessageReplyTo {
  InputMessageReplyTo::inputMessageReplyToMessage(types::inputMessageReplyToMessage { message_id: mid, ..Default::default() })
}

fn formatted_text(text: impl Into<String>) -> types::formattedText {
  types::formattedText { text: text.into(), ..Default::default() }
}

fn text_content(text: impl Into<String>) -> InputMessageContent {
  InputMessageContent::inputMessageText(types::inputMessageText { text: formatted_text(text), ..Default::default() })
}

fn document_content(document: InputFile, caption: Option<String>) -> InputMessageContent {
  let document = types::inputDocument { document, ..Default::default() };
  let caption = caption.map(formatted_text);
  InputMessageContent::inputMessageDocument(types::inputMessageDocument { document, caption })
}
