//! Representative composition contracts, independent of Telegram credentials.

use std::assert_matches;

use anyhow::{Result, bail};
use serde_json::json;
use tdx::enums::{InputPageBlock, RichMessageSource, RichText, TextEntityType};
use tdx::format::*;
use tdx::{compose::file, types};

#[test]
fn nested_text_keeps_utf16_offsets_when_borrowed_moved_and_joined() {
  use TextEntityType::{textEntityTypeBold as Bold, textEntityTypeItalic as Italic};

  let fragment = line(bold(italic("🦀é"))) + underline("");
  let text = lines([line("Hi ") + &fragment, empty(), fragment]);
  assert_eq!(&*text, "Hi 🦀é\n\n🦀é");

  let spans: Vec<_> = text.entities().iter().map(|entity| (entity.offset, entity.length, &entity.r#type)).collect();
  assert_eq!(spans, [(3, 3, &Italic), (3, 3, &Bold), (8, 3, &Italic), (8, 3, &Bold)]);

  let native: types::formattedText = text.clone().into();
  assert_eq!(native.text(), text);
  assert_eq!(lines([] as [Text; 0]), empty());
  assert_eq!(lines([bold("One"), italic("Two")]), line(bold("One")) + "\n" + italic("Two"));
}

#[test]
fn display_parts_stream_into_surrounding_styles() {
  let mut text = line("Weight: ");
  text += bold(format_args!("{:.2} kg", 1.5));
  text.entity(TextEntityType::textEntityTypeCode, |text| {
    text.push(" / ");
    text.push(42);
  });
  assert_eq!(&*text, "Weight: 1.50 kg / 42");
  let entities = text.entities();
  assert_matches!(
    entities,
    [
      types::textEntity { offset: 8, length: 7, r#type: TextEntityType::textEntityTypeBold },
      types::textEntity { offset: 15, length: 5, r#type: TextEntityType::textEntityTypeCode },
    ]
  );
}

#[test]
fn link_wire_round_trips_with_utf16_positions() {
  let native: types::formattedText = (line("🦀 ") + link("Open", "https://example.com")).into();
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
fn rich_styles_keep_nesting_targets_and_emoji_fallbacks() -> Result<()> {
  let nested = plain(bold(italic("Catch")));
  let RichText::richTextBold(outer) = nested else { bail!("expected bold") };
  assert_matches!(*outer.text, RichText::richTextItalic(_));

  let command = plain(bot_command_target("Cast again", "/fish"));
  assert_matches!(command, RichText::richTextBotCommand(ref value) if value.bot_command == "/fish");
  let emoji = plain(custom_emoji("🎣", 123));
  assert_matches!(emoji, RichText::richTextCustomEmoji(ref value) if value.alternative_text == "🎣");

  let label = concat([plain("Weight: "), plain(code(format_args!("{} kg", 2)))]);
  assert_matches!(label, RichText::richTexts(ref value) if value.texts.len() == 2);
  assert_eq!(concat(["Only item"]), plain("Only item"));
  Ok(())
}

#[test]
fn native_tables_keep_layout_and_remain_editable() -> Result<()> {
  use tdx::enums::PageBlockHorizontalAlignment::pageBlockHorizontalAlignmentRight;

  let mut report = table(["Species", "Weight"]) //.
    .bordered()
    .row([cell(bold("Trout")), cell("1.5 kg").right()]);
  report.is_compact = true;
  assert!(report.is_bordered && report.is_compact);
  let [headers, row] = report.cells.as_slice() else { bail!("expected headers and one row") };
  assert!(headers.iter().all(|cell| cell.is_header));
  let [_, weight] = row.as_slice() else { bail!("expected two cells") };
  assert_eq!(weight.align, pageBlockHorizontalAlignmentRight);

  let spanning = cell("Summary").span(2, 3);
  assert_eq!((spanning.colspan, spanning.rowspan), (2, 3));
  assert_eq!(table([] as [&str; 0]).cells, Vec::<Vec<Cell>>::new());
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
    header: plain("Notes"), //.
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
  let expected: types::formattedText = (line("🦀 ") + bold("bold") + " " + code("code")).into();
  assert_eq!(markdown("🦀 **bold** `code`").unwrap(), expected);
  assert_eq!(html("🦀 <b>bold</b> <code>code</code>").unwrap(), expected);
  assert_eq!(html("a &amp; b").unwrap().text, "a & b");
  assert!(html("<b>unclosed").is_err());
}
