// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt;

pub type Result<T, E = EngineError> = std::result::Result<T, E>;

/// The subset of the upstream engine error contract that the vendored
/// handwriting module uses. Variant names and `Display` text are kept so the
/// module and its tests compile unchanged.
#[derive(Debug)]
pub enum EngineError {
    InvalidArgument(String),
    Failed(String),
}

impl EngineError {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::InvalidArgument(message.into())
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self::Failed(message.into())
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument(message) | Self::Failed(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for EngineError {}
