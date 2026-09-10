//! Inline styles shared by entity text and rich text trees.
//!
//! A [`Styled`] value keeps its content until consumed. Nest styles directly;
//! tuples compose heterogeneous styled and unstyled fragments in both ordinary
//! and rich text.
//! URL, language and command arguments stay borrowed until rendering.
//! Ordinary text allocates only the metadata its entities actually use.

use td_types::enums::{DateTimeFormattingType, RichText, TextEntityType};
use td_types::types;

use super::{IntoRichText, Part, Text, line};

macro_rules! define_styles {
  ($($(#[$meta:meta])* $name:ident($($arg:ident: $ty:ty),*) => $kind:ident,
    entity = $entity:expr, rich($inner:ident) = $rich:expr);* $(;)?) => {
    #[derive(Debug, Clone, PartialEq)]
    enum Style<'a> {
      $($kind($($ty),*),)*
      CustomEmoji(i64, &'a str),
    }

    impl Style<'_> {
      fn into_entity_type(self) -> TextEntityType {
        match self {
          $(Self::$kind($($arg),*) => { $(let _ = &$arg;)* $entity })*
          Self::CustomEmoji(custom_emoji_id, _) => types::textEntityTypeCustomEmoji { custom_emoji_id }.into(),
        }
      }

      fn into_rich_text(self, content: impl IntoRichText) -> RichText {
        match self {
          $(Self::$kind($($arg),*) => {
            $(let _ = &$arg;)*
            let $inner = content.into_rich_text();
            $rich
          })*
          Self::CustomEmoji(custom_emoji_id, alternative_text) => {
            types::richTextCustomEmoji { custom_emoji_id, alternative_text: alternative_text.into() }.into()
          }
        }
      }
    }

    $($(#[$meta])*
    #[must_use]
    pub const fn $name<'a, T>(content: T, $($arg: $ty),*) -> Styled<'a, T> {
      Styled { kind: Style::$kind($($arg),*), content }
    })*
  };
}

define_styles! {
  /// Applies bold.
  bold() => Bold,
  entity = TextEntityType::textEntityTypeBold,
  rich(inner) = types::richTextBold { text: Box::new(inner) }.into();

  /// Applies italic.
  italic() => Italic,
  entity = TextEntityType::textEntityTypeItalic,
  rich(inner) = types::richTextItalic { text: Box::new(inner) }.into();

  /// Applies underline.
  underline() => Underline,
  entity = TextEntityType::textEntityTypeUnderline,
  rich(inner) = types::richTextUnderline { text: Box::new(inner) }.into();

  /// Applies strikethrough.
  strike() => Strike,
  entity = TextEntityType::textEntityTypeStrikethrough,
  rich(inner) = types::richTextStrikethrough { text: Box::new(inner) }.into();

  /// Applies spoiler.
  spoiler() => Spoiler,
  entity = TextEntityType::textEntityTypeSpoiler,
  rich(inner) = types::richTextSpoiler { text: Box::new(inner) }.into();

  /// Applies fixed-width inline code.
  code() => Code,
  entity = TextEntityType::textEntityTypeCode,
  rich(inner) = types::richTextFixed { text: Box::new(inner) }.into();

  /// Uses a command target in rich text; ordinary text marks the visible command itself.
  bot_command_target(command: &'a str) => BotCommand,
  entity = TextEntityType::textEntityTypeBotCommand,
  rich(inner) = types::richTextBotCommand { text: Box::new(inner), bot_command: command.into() }.into();

  /// Adds a language-tagged code entity; rich text retains only fixed-width styling.
  pre(language: &'a str) => Pre,
  entity = types::textEntityTypePreCode { language: language.into() }.into(),
  rich(inner) = types::richTextFixed { text: Box::new(inner) }.into();

  /// Marks the content as a clickable URL link.
  link(url: &'a str) => Link,
  entity = types::textEntityTypeTextUrl { url: url.into() }.into(),
  rich(inner) = types::richTextUrl { text: Box::new(inner), url: url.into(), is_cached: false }.into();

  /// Marks the content as a user mention by numeric Telegram ID.
  mention(user_id: i64) => Mention,
  entity = types::textEntityTypeMentionName { user_id }.into(),
  rich(inner) = types::richTextMentionName { text: Box::new(inner), user_id }.into();

  /// Marks the content as a dynamic date/time entity.
  time(unix_time: i32, format: Option<DateTimeFormattingType>) => Time,
  entity = types::textEntityTypeDateTime { unix_time, formatting_type: format }.into(),
  rich(inner) = types::richTextDateTime { text: Box::new(inner), unix_time, formatting_type: format }.into();

  /// Adds a seek entity in ordinary text; rich text retains the content unchanged.
  media_timestamp(timestamp_seconds: i32) => MediaTimestamp,
  entity = types::textEntityTypeMediaTimestamp { media_timestamp: timestamp_seconds }.into(),
  rich(inner) = inner;

  /// Adds an ordinary-text blockquote; use `block_quote` for rich blocks.
  quote() => BlockQuote,
  entity = TextEntityType::textEntityTypeBlockQuote,
  rich(inner) = inner;

  /// Adds an expandable text entity; use `block_quote_expandable` for rich blocks.
  quote_expandable() => ExpandableBlockQuote,
  entity = TextEntityType::textEntityTypeExpandableBlockQuote,
  rich(inner) = inner;
}

/// Marks a command whose visible text is also its rich-text target.
#[must_use]
pub const fn bot_command(command: &str) -> Styled<'_, &str> {
  bot_command_target(command, command)
}

/// Marks a timestamp for Telegram's client-updated relative rendering.
///
/// The visible content is the fallback text clients can show while resolving the
/// date entity. Supporting clients update relative values in place without bot
/// message edits.
#[must_use]
pub const fn relative_time<T>(content: T, unix_time: i32) -> Styled<'static, T> {
  time(content, unix_time, Some(DateTimeFormattingType::dateTimeFormattingTypeRelative))
}

/// Borrows the custom emoji fallback until either output is rendered.
#[must_use]
pub const fn custom_emoji(content: &str, custom_emoji_id: i64) -> Styled<'_, &str> {
  Styled { kind: Style::CustomEmoji(custom_emoji_id, content), content }
}

/// Content with a deferred style, rendered when appended or converted.
///
/// Nested spans write into the destination [`Text`] buffer. Rich conversion
/// allocates the corresponding generated tree nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct Styled<'a, T> {
  kind: Style<'a>,
  content: T,
}

impl<T: Part> Part for Styled<'_, T> {
  fn write_to(self, text: &mut Text) {
    text.entity(self.kind.into_entity_type(), |text| self.content.write_to(text));
  }
}

impl<T: IntoRichText> IntoRichText for Styled<'_, T> {
  fn into_rich_text(self) -> RichText {
    self.kind.into_rich_text(self.content)
  }

  fn append_to(self, texts: &mut Vec<RichText>) {
    texts.push(self.into_rich_text());
  }
}

impl<T: Part> From<Styled<'_, T>> for Text {
  fn from(value: Styled<'_, T>) -> Self {
    line(value)
  }
}
