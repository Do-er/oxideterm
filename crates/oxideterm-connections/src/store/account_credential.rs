// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use crate::account_credential::{
    ACCOUNT_CREDENTIAL_KEYCHAIN_PREFIX, AccountCredential, AccountCredentialError,
};

impl ConnectionStore {
    pub fn account_credentials(&self) -> &[AccountCredential] {
        &self.data.account_credentials
    }

    pub fn get_account_credential(&self, id: &str) -> Option<&AccountCredential> {
        self.data.account_credentials.iter().find(|cred| cred.id == id)
    }

    pub fn count_account_credential_references(&self, id: &str) -> usize {
        self.data
            .connections
            .iter()
            .filter(|conn| conn.options.account_credential_id.as_deref() == Some(id))
            .count()
    }

    pub fn get_account_credential_password(
        &self,
        id: &str,
    ) -> Result<SecretString, AccountCredentialError> {
        let cred = self
            .get_account_credential(id)
            .ok_or(AccountCredentialError::NotFound)?;
        let keychain_id = cred
            .keychain_id
            .as_deref()
            .ok_or(AccountCredentialError::MissingPassword)?;
        self.keychain
            .get(keychain_id)
            .map_err(AccountCredentialError::Storage)
    }

    pub fn save_account_credential(
        &mut self,
        id: Option<&str>,
        name: &str,
        username: &str,
        password: Option<&SecretString>,
    ) -> Result<String, AccountCredentialError> {
        let name = name.trim();
        if name.is_empty() || name.len() > 256 {
            return Err(AccountCredentialError::InvalidName);
        }
        let username = username.trim();
        if username.is_empty() || username.len() > 256 {
            return Err(AccountCredentialError::InvalidUsername);
        }

        let existing = id
            .and_then(|id| self.data.account_credentials.iter().find(|c| c.id == id))
            .cloned();
        if id.is_some() && existing.is_none() {
            return Err(AccountCredentialError::NotFound);
        }
        if existing.is_none() && password.is_none_or(|p| p.is_empty()) {
            return Err(AccountCredentialError::MissingPassword);
        }

        let credential_id = id
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let keychain_id = if let Some(pass) = password.filter(|p| !p.is_empty()) {
            let kid = existing
                .as_ref()
                .and_then(|c| c.keychain_id.clone())
                .unwrap_or_else(|| format!("{}{}", ACCOUNT_CREDENTIAL_KEYCHAIN_PREFIX, credential_id));
            self.keychain
                .store(&kid, pass)
                .map_err(AccountCredentialError::Storage)?;
            Some(kid)
        } else {
            existing.as_ref().and_then(|c| c.keychain_id.clone())
        };

        let now = Utc::now();
        let credential = AccountCredential {
            id: credential_id.clone(),
            name: name.to_string(),
            username: username.to_string(),
            keychain_id,
            created_at: existing.as_ref().map(|c| c.created_at).unwrap_or(now),
            updated_at: now,
        };
        credential.validate()?;

        if let Some(pos) = self
            .data
            .account_credentials
            .iter()
            .position(|c| c.id == credential_id)
        {
            self.data.account_credentials[pos] = credential;
        } else {
            self.data.account_credentials.push(credential);
        }

        self.normalize();
        self.save().map_err(AccountCredentialError::Storage)?;
        Ok(credential_id)
    }

    pub fn delete_account_credential(&mut self, id: &str) -> Result<(), AccountCredentialError> {
        let pos = self
            .data
            .account_credentials
            .iter()
            .position(|c| c.id == id)
            .ok_or(AccountCredentialError::NotFound)?;
        let removed = self.data.account_credentials.remove(pos);

        if let Some(keychain_id) = removed.keychain_id {
            let _ = self.keychain.delete(&keychain_id);
        }

        for conn in &mut self.data.connections {
            if conn.options.account_credential_id.as_deref() == Some(id) {
                conn.options.account_credential_id = None;
            }
        }

        self.normalize();
        self.save().map_err(AccountCredentialError::Storage)?;
        Ok(())
    }
}
