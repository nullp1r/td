//! Telegram MMO RPG game engine and daemon.

mod app;
mod content;
mod db;
mod fishing;
mod ids;
mod rng;
mod timer;
mod view;
mod world;

#[cfg(feature = "telegram")]
/// Telegram bot presentation, interaction, and background drivers.
pub mod telegram;
