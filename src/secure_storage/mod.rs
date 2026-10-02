// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Secure storage module for encrypting sensitive data and password hashing.
//!
//! - **encryption**: AES-256-GCM encryption for private keys with machine-derived keys.
//!   Data can only be decrypted on the same machine.
//! - **password**: BLAKE3 password hashing with salt for lockscreen authentication.

mod encryption;
mod error;
mod password;

pub use encryption::*;
pub use error::{Error, Result};
pub use password::*;
