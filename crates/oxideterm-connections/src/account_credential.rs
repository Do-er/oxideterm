// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const ACCOUNT_CREDENTIAL_KEYCHAIN_PREFIX: &str = "oxide_account_credential_";

pub fn is_account_credential_keychain_id(id: &str) -> bool {
    id.starts_with(ACCOUNT_CREDENTIAL_KEYCHAIN_PREFIX)
}

#[derive(Debug, Error)]
pub enum AccountCredentialError {
    #[error("Credential name is required and cannot exceed 256 characters")]
    InvalidName,
    #[error("Username is required and cannot exceed 256 characters")]
    InvalidUsername,
    #[error("Password is required for a new credential")]
    MissingPassword,
    #[error("Account credential not found")]
    NotFound,
    #[error(transparent)]
    Storage(#[from] anyhow::Error),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccountCredential {
    pub id: String,
    pub name: String,
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keychain_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AccountCredential {
    pub fn portable(&self) -> Self {
        Self {
            keychain_id: None,
            ..self.clone()
        }
    }

    pub fn validate(&self) -> Result<(), AccountCredentialError> {
        if self.id.trim().is_empty() {
            return Err(AccountCredentialError::NotFound);
        }
        let name = self.name.trim();
        if name.is_empty() || name.len() > 256 {
            return Err(AccountCredentialError::InvalidName);
        }
        let username = self.username.trim();
        if username.is_empty() || username.len() > 256 {
            return Err(AccountCredentialError::InvalidUsername);
        }
        Ok(())
    }
}

