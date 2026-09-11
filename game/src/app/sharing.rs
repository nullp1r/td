//! Immutable catch projections used by Telegram inline sharing.

use rusqlite::OptionalExtension as _;

use super::{App, Result, require_character_id};
use crate::{
  ids::{LocationId, SpeciesId},
  view::InlineCatchView,
};

const INLINE_CATCH_LIMIT: usize = 20;

impl App {
  pub async fn inline_catches(&self, telegram_user_id: i64, query: String) -> Result<Vec<InlineCatchView>> {
    let content = self.content.clone();
    self
      .run_db(move |connection| -> Result<_> {
        let character_id = require_character_id(connection, telegram_user_id)?;
        let exact_item_id = query.strip_prefix("catch:").and_then(|value| value.trim().parse::<i64>().ok());
        let mut statement = if exact_item_id.is_some() {
          connection.prepare(
            "SELECT c.item_id, c.species_id, c.length_mm, c.weight_g, c.location_id, ch.name
             FROM catches c JOIN items i ON i.id = c.item_id JOIN characters ch ON ch.id = i.owner_character_id
             WHERE i.owner_character_id = ?1 AND c.item_id = ?2",
          )?
        } else {
          connection.prepare(
            "SELECT c.item_id, c.species_id, c.length_mm, c.weight_g, c.location_id, ch.name
             FROM catches c JOIN items i ON i.id = c.item_id JOIN characters ch ON ch.id = i.owner_character_id
             WHERE i.owner_character_id = ?1 ORDER BY c.caught_at_ms DESC, c.item_id DESC LIMIT 64",
          )?
        };

        let mut raw = Vec::new();
        if let Some(item_id) = exact_item_id {
          if let Some(row) = statement
            .query_row([character_id, item_id], |row| {
              Ok((
                row.get::<_, i64>(0)?,
                SpeciesId(row.get(1)?),
                row.get::<_, u32>(2)?,
                row.get::<_, u32>(3)?,
                LocationId(row.get(4)?),
                row.get::<_, String>(5)?,
              ))
            })
            .optional()?
          {
            raw.push(row);
          }
        } else {
          let rows = statement.query_map([character_id], |row| {
            Ok((row.get::<_, i64>(0)?, SpeciesId(row.get(1)?), row.get::<_, u32>(2)?, row.get::<_, u32>(3)?, LocationId(row.get(4)?), row.get::<_, String>(5)?))
          })?;
          raw.extend(rows.collect::<rusqlite::Result<Vec<_>>>()?);
        }
        drop(statement);

        let filter = query.trim().to_ascii_lowercase();
        let filter = (!filter.is_empty() && exact_item_id.is_none()).then_some(filter);
        let mut catches = Vec::new();
        for (item_id, species_id, length_mm, weight_g, location_id, owner_name) in raw {
          let species = content.species(species_id);
          if filter.as_ref().is_some_and(|filter| !species.name.to_ascii_lowercase().contains(filter)) {
            continue;
          }
          let personal_best: u32 = connection.query_row(
            "SELECT max(c.weight_g) FROM catches c JOIN items i ON i.id = c.item_id WHERE i.owner_character_id = ?1 AND c.species_id = ?2",
            rusqlite::params![character_id, species_id.0],
            |row| row.get(0),
          )?;
          let world_best: u32 = connection.query_row("SELECT max(weight_g) FROM catches WHERE species_id = ?1", [species_id.0], |row| row.get(0))?;
          catches.push(InlineCatchView {
            item_id,
            species_name: species.name.clone(),
            length_mm,
            weight_g,
            location_name: content.location(location_id).name.clone(),
            owner_name,
            personal_best: weight_g == personal_best,
            world_best: weight_g == world_best,
          });
          if catches.len() == INLINE_CATCH_LIMIT {
            break;
          }
        }
        Ok(catches)
      })
      .await
  }
}
