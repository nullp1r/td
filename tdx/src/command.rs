//! Entity-aware parsing for Telegram bot commands.

use td_types::enums::MessageContent;
use td_types::enums::TextEntityType::textEntityTypeBotCommand as EntityCommand;
use td_types::types;

use crate::ext::ContentExt as _;
use crate::util::Utf16 as _;

/// A parsed bot command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Command<'a> {
  /// The command name without its leading slash or optional bot mention.
  pub name: &'a str,
  /// The bot username following `@`, if targeted to a specific bot.
  pub target: Option<&'a str>,
  /// The remainder of the message with surrounding whitespace trimmed.
  pub args: &'a str,
}

impl<'a> Command<'a> {
  /// Reports whether this command is unaddressed or targeted at `username`.
  pub fn is_for(&self, username: Option<&str>) -> bool {
    match [self.target, username] {
      [Some(target), Some(username)] => target.eq_ignore_ascii_case(username),
      [None, _] => true,
      _ => false,
    }
  }

  fn split(head: &'a str, args: &'a str) -> Self {
    let (name, target) = match head.split_once('@') {
      Some((name, target)) if !target.is_empty() => (name, Some(target)),
      Some((name, _)) => (name, None),
      None => (head, None),
    };
    Self { name, target, args: args.trim() }
  }
}

/// Extracts a bot command from messages, formatted text, or plain text.
///
/// Messages and formatted text require a bot-command entity at offset zero.
/// Plain strings only split the leading slash token; they do not validate Telegram
/// command names. No form checks operator permissions or the bot's registered commands.
pub trait CommandExt {
  /// Extracts a bot command, if present.
  fn command(&self) -> Option<Command<'_>>;

  /// Extracts a bot command targeted at `username` or unaddressed.
  fn command_for(&self, username: Option<&str>) -> Option<Command<'_>> {
    self.command().filter(|cmd| cmd.is_for(username))
  }
}

impl CommandExt for types::message {
  fn command(&self) -> Option<Command<'_>> {
    self.content.command()
  }
}

impl CommandExt for MessageContent {
  fn command(&self) -> Option<Command<'_>> {
    self.text()?.command()
  }
}

impl CommandExt for types::formattedText {
  fn command(&self) -> Option<Command<'_>> {
    let entity = self.entities.iter().find(|e| matches!((&e.r#type, e.offset), (EntityCommand, 0)))?;
    let (head, args) = self.text.split_at_utf16(entity.length as usize)?;
    let head = head.strip_prefix('/')?;
    Some(Command::split(head, args))
  }
}

impl CommandExt for str {
  fn command(&self) -> Option<Command<'_>> {
    let text = self.strip_prefix('/')?;
    let (head, args) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    let 1.. = head.len() else { return None };
    Some(Command::split(head, args))
  }
}
