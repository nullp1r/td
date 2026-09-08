//! Callback query answer requests.

use std::fmt::Display;

use td_types::fns;

use crate::target::CallbackQueryTarget;

/// Constructs a callback query answer.
///
/// Accepts a query ID or any type implementing [`CallbackQueryTarget`].
/// Accepts plain text, `String`, or `format_args!(...)`.
/// URL and cache-time fields remain editable on the returned request.
pub fn answer(target: &impl CallbackQueryTarget, text: impl Display, show_alert: bool) -> fns::answerCallbackQuery {
  let (callback_query_id, text) = (target.callback_query_id(), text.to_string());
  fns::answerCallbackQuery { callback_query_id, text, show_alert, ..Default::default() }
}

/// Acknowledges a callback query without displaying text to dismiss the button loading spinner.
pub fn ack(target: &impl CallbackQueryTarget) -> fns::answerCallbackQuery {
  answer(target, "", false)
}

/// Answers a callback query with a discreet toast notification banner.
pub fn toast(target: &impl CallbackQueryTarget, text: impl Display) -> fns::answerCallbackQuery {
  answer(target, text, false)
}

/// Answers a callback query with an explicit modal alert popup dialog.
pub fn alert(target: &impl CallbackQueryTarget, text: impl Display) -> fns::answerCallbackQuery {
  answer(target, text, true)
}
