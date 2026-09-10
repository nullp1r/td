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

/// Telegram bot presentation, interaction, and background drivers.
#[cfg(feature = "telegram")]
pub mod telegram;
