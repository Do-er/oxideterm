// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

//! Display-name resolution for saved connection assets.
//!
//! The stored name is optional: a user may leave it empty so the label keeps
//! following the endpoint. Every surface (list rows, tab titles, API snapshots)
//! resolves the same fallback through these methods instead of inventing one.

use std::borrow::Cow;

use crate::store::{
    FtpProfile, MoshProfile, RemoteDesktopProfile, SavedConnection, SerialProfile,
    StandaloneSftpProfile, TelnetProfile,
};

/// Brackets an IPv6 literal so it stays readable next to a port.
pub fn endpoint_host_label(host: &str) -> Cow<'_, str> {
    let host = host.trim();
    if host.contains(':') && !host.starts_with('[') {
        Cow::Owned(format!("[{host}]"))
    } else {
        Cow::Borrowed(host)
    }
}

fn resolved<'a>(name: &'a str, fallback: impl FnOnce() -> Cow<'a, str>) -> Cow<'a, str> {
    let name = name.trim();
    if name.is_empty() {
        fallback()
    } else {
        Cow::Borrowed(name)
    }
}

impl SavedConnection {
    pub fn display_name(&self) -> Cow<'_, str> {
        resolved(&self.name, || endpoint_host_label(&self.host))
    }
}

impl MoshProfile {
    pub fn display_name(&self) -> Cow<'_, str> {
        resolved(&self.name, || endpoint_host_label(&self.host))
    }
}

impl StandaloneSftpProfile {
    pub fn display_name(&self) -> Cow<'_, str> {
        resolved(&self.name, || endpoint_host_label(&self.host))
    }
}

impl FtpProfile {
    pub fn display_name(&self) -> Cow<'_, str> {
        resolved(&self.name, || endpoint_host_label(&self.host))
    }
}

impl TelnetProfile {
    pub fn display_name(&self) -> Cow<'_, str> {
        resolved(&self.name, || endpoint_host_label(&self.host))
    }
}

impl RemoteDesktopProfile {
    pub fn display_name(&self) -> Cow<'_, str> {
        resolved(&self.name, || endpoint_host_label(&self.host))
    }
}

impl SerialProfile {
    pub fn display_name(&self) -> Cow<'_, str> {
        resolved(&self.name, || Cow::Borrowed(self.port_path.trim()))
    }
}
