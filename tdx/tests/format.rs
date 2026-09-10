//! Representative composition contracts, independent of Telegram credentials.

use std::assert_matches;

use anyhow::{Result, bail};
use serde_json::json;
use tdx::enums::{DateTimeFormattingType, InputPageBlock, RichMessageSource, RichText, TextEntityType};
use tdx::format::*;
use tdx::{compose::file, types};

#[test]
fn tuple_text_keeps_utf16_offsets_when_borrowed_moved_and_joined() {
  use TextEntityType::{textEntityTypeBold as Bold, textEntityTypeItalic as Italic};

  let fragment = line(bold(italic("🦀é")));
  let moved = fragment.clone();
  let text = lines((("Hi ", &fragment), "", moved));
  assert_eq!(&*text, "Hi 🦀é\n\n🦀é");

  let spans: Vec<_> = text.entities().iter().map(|entity| (entity.offset, entity.length, &entity.r#type)).collect();
  assert_eq!(spans, [(3, 3, &Italic), (3, 3, &Bold), (8, 3, &Italic), (8, 3, &Bold)]);

  let native: types::formattedText = text.clone().into();
  assert_eq!(native.text(), text);
  assert_eq!(lines([] as [&str; 0]), Text::new());
  assert_eq!(lines((bold("One"), italic("Two"))), line((bold("One"), "\n", italic("Two"))));
  assert_eq!(lines(vec!["One", "Two"]), line(("One", "\n", "Two")));
}

#[test]
fn scalar_collection_and_nested_parts_stream_into_one_text() {
  let mut text = line(("Weight: ", bold(format_args!("{:.2} kg", 1.5)), [" / ", "count "]));
  text.push(42);
  text.push(vec![" / ", "vec"]);
  text.entity(TextEntityType::textEntityTypeCode, |text| text.push((" · ", 7_u8)));
  assert_eq!(&*text, "Weight: 1.50 kg / count 42 / vec · 7");
  let entities = text.entities();
  assert_matches!(
    entities,
    [
      types::textEntity { offset: 8, length: 7, r#type: TextEntityType::textEntityTypeBold },
      types::textEntity { offset: 32, length: 4, r#type: TextEntityType::textEntityTypeCode },
    ]
  );

  let nested = line(bold(("A", italic("B"), "C")));
  assert_eq!(&*nested, "ABC");
  assert_matches!(
    nested.entities(),
    [
      types::textEntity { offset: 1, length: 1, r#type: TextEntityType::textEntityTypeItalic },
      types::textEntity { offset: 0, length: 3, r#type: TextEntityType::textEntityTypeBold },
    ]
  );
}

#[test]
fn link_wire_round_trips_with_utf16_positions() {
  let native: types::formattedText = line(("🦀 ", link("Open", "https://example.com"))).into();
  let wire = serde_json::to_value(&native).unwrap();
  assert_eq!(
    wire,
    json!({
      "text": "🦀 Open",
      "entities": [{
        "offset": 3, "length": 4,
        "type": {"@type": "textEntityTypeTextUrl", "url": "https://example.com"}
      }]
    })
  );
  assert_eq!(serde_json::from_value::<types::formattedText>(wire).unwrap(), native);
}

#[test]
fn rich_tuples_flatten_while_styles_keep_nesting_targets_and_fallbacks() -> Result<()> {
  let nested = bold(italic("Catch")).into_rich_text();
  let RichText::richTextBold(outer) = nested else { bail!("expected bold") };
  assert_matches!(*outer.text, RichText::richTextItalic(_));

  let composed_style = bold(("A", italic("B"), "C")).into_rich_text();
  let RichText::richTextBold(outer) = composed_style else { bail!("expected composed bold") };
  assert_matches!(*outer.text, RichText::richTexts(ref value) if value.texts.len() == 3);

  let command = bot_command_target("Cast again", "/fish").into_rich_text();
  assert_matches!(command, RichText::richTextBotCommand(ref value) if value.bot_command == "/fish");
  let emoji = custom_emoji("🎣", 123).into_rich_text();
  assert_matches!(emoji, RichText::richTextCustomEmoji(ref value) if value.alternative_text == "🎣");

  let label = ("Weight: ", ("net ", code(format_args!("{}", 2)), " kg")).into_rich_text();
  assert_matches!(label, RichText::richTexts(ref value) if value.texts.len() == 4);
  assert_eq!(["Only item"].into_rich_text(), "Only item".into_rich_text());
  assert_matches!(vec!["a", "b"].into_rich_text(), RichText::richTexts(ref value) if value.texts.len() == 2);
  Ok(())
}

#[test]
fn relative_time_uses_native_client_updated_formatting() -> Result<()> {
  let timestamp = 1_800_000_000;
  let rich = relative_time("soon", timestamp).into_rich_text();
  let RichText::richTextDateTime(value) = rich else { bail!("expected rich date/time") };
  assert_eq!(value.unix_time, timestamp);
  assert_matches!(value.formatting_type, Some(DateTimeFormattingType::dateTimeFormattingTypeRelative));

  let ordinary = line(relative_time("soon", timestamp));
  assert_matches!(
    ordinary.entities(),
    [types::textEntity {
      offset: 0,
      length: 4,
      r#type: TextEntityType::textEntityTypeDateTime(types::textEntityTypeDateTime {
        unix_time,
        formatting_type: Some(DateTimeFormattingType::dateTimeFormattingTypeRelative),
      }),
    }] if *unix_time == timestamp
  );
  Ok(())
}

#[test]
fn native_tables_support_optional_headers_and_heterogeneous_rows() -> Result<()> {
  use tdx::enums::PageBlockHorizontalAlignment::pageBlockHorizontalAlignmentRight;

  let mut report = table() //.
    .header(("Species", "Weight"))
    .bordered()
    .row((bold("Trout"), cell("1.5 kg").right()));
  report.is_compact = true;
  assert!(report.is_bordered && report.is_compact);
  let [headers, row] = report.cells.as_slice() else { bail!("expected headers and one row") };
  assert!(headers.iter().all(|cell| cell.is_header));
  let [_, weight] = row.as_slice() else { bail!("expected two cells") };
  assert_eq!(weight.align, pageBlockHorizontalAlignmentRight);

  let spanning = cell("Summary").span(2, 3);
  assert_eq!((spanning.colspan, spanning.rowspan), (2, 3));
  assert!(table().cells.is_empty());
  let headerless = table().row(("🐟 Catches", 3_u32));
  let [row] = headerless.cells.as_slice() else { bail!("expected one row") };
  assert!(row.iter().all(|cell| !cell.is_header));
  Ok(())
}

#[test]
fn lists_apply_numbering_without_losing_nested_blocks_or_checkboxes() -> Result<()> {
  let nested = ListItem { blocks: vec![paragraph("Read the guide"), divider()], ..Default::default() };
  let items = [list_item("Cast a line").checked(true), nested];
  let ordered = ordered_list_styled("a", items.clone());
  let InputPageBlock::inputPageBlockList(list) = ordered else { bail!("expected a list") };
  let numbering: Vec<_> = list.items.iter().map(|item| (item.value, item.r#type.as_str())).collect();
  assert_eq!(numbering, [(1, "a"), (2, "a")]);
  let [first, second] = list.items.as_slice() else { bail!("expected two items") };
  assert!(first.has_checkbox && first.is_checked);
  assert_eq!(second.blocks.len(), 2);

  let bullets = bullet_list(items);
  let InputPageBlock::inputPageBlockList(list) = bullets else { bail!("expected a list") };
  assert!(list.items.iter().all(|item| item.value == 0 && item.r#type.is_empty()));
  Ok(())
}

#[test]
fn documents_preserve_block_order_and_accept_native_options() -> Result<()> {
  let mut notes = types::inputPageBlockDetails {
    header: "Notes".into_rich_text(), //.
    blocks: vec![paragraph("Fresh today")],
    ..Default::default()
  };
  notes.is_open = true;
  let message = rich([
    heading(bold("Lake report"), 1), //.
    video(file::local("catch.mp4")),
    notes.into(),
  ]);
  let RichMessageSource::richMessageSourceBlocks(source) = message.message.source else {
    bail!("expected a document made of blocks");
  };
  let blocks = source.blocks.as_slice();
  assert_matches!(blocks, [
    InputPageBlock::inputPageBlockSectionHeading(_),
    InputPageBlock::inputPageBlockVideo(video),
    InputPageBlock::inputPageBlockDetails(notes),
  ] if video.video.supports_streaming && notes.is_open);
  Ok(())
}

#[test]
fn native_parsers_agree_with_composition_and_reject_bad_html() {
  let expected: types::formattedText = line(("🦀 ", bold("bold"), " ", code("code"))).into();
  assert_eq!(markdown("🦀 **bold** `code`").unwrap(), expected);
  assert_eq!(html("🦀 <b>bold</b> <code>code</code>").unwrap(), expected);
  assert_eq!(html("a &amp; b").unwrap().text, "a & b");
  assert!(html("<b>unclosed").is_err());
}

#[test]
fn rich_button_rows_keep_styles_and_callback_payloads() -> Result<()> {
  use tdx::enums::{ButtonStyle, InlineKeyboardButtonType};

  let row = button_row([success_callback_button("Cast", b"cast"), primary_callback_button("Explore", b"explore"), danger_callback_button("Cut", b"cut")]);
  let InputPageBlock::inputPageBlockButtonRow(row) = row else { bail!("expected rich button row") };
  let [cast, explore, cut] = row.buttons.as_slice() else { bail!("expected three buttons") };
  assert_eq!(cast.style, ButtonStyle::buttonStyleSuccess);
  assert_eq!(explore.style, ButtonStyle::buttonStylePrimary);
  assert_eq!(cut.style, ButtonStyle::buttonStyleDanger);
  let InlineKeyboardButtonType::inlineKeyboardButtonTypeCallback(callback) = &cast.r#type else {
    bail!("expected callback button");
  };
  assert_eq!(callback.data, b"cast");
  let RichText::richTextPlain(text) = cast.text.as_ref() else { bail!("expected plain button label") };
  assert_eq!(text.text, "Cast");
  Ok(())
}
