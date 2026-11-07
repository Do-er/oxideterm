// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use oxideterm_settings::UpdateChannel;

// Update channels resolve against the fork's own releases. Never point these at
// AnalyseDeCircuit/oxideterm: a fork build must not be able to update itself to
// an official OxideTerm package.
pub const STABLE_UPDATE_ENDPOINT: &str =
    "https://github.com/Do-er/oxideterm/releases/latest/download/latest.json";
// Beta uses a fork-owned pinned tag hosting latest.json; create the release with
// that tag before offering the beta channel, otherwise checks fail with HTTP 404.
pub const BETA_UPDATE_ENDPOINT: &str =
    "https://github.com/Do-er/oxideterm/releases/download/updater-beta/latest.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UpdateEndpoint {
    pub channel: UpdateChannel,
    pub url: &'static str,
}

pub fn endpoint_for_channel(channel: UpdateChannel) -> UpdateEndpoint {
    let url = match channel {
        UpdateChannel::Stable => STABLE_UPDATE_ENDPOINT,
        UpdateChannel::Beta => BETA_UPDATE_ENDPOINT,
    };
    UpdateEndpoint { channel, url }
}
