//! `cloud-spare-warm` (`cloud-spare-warm-v1`): the app's Cloud intent (it came to the front
//! signed in, the composer picked Cloud, a reserve or a claim named a machine) as one daemon
//! command. Leo, 2026-11-10, about the Swift warm that froze NIGHTLY's main thread in a pipe
//! write: "Why is this even in Swift anyways mang".
//!
//! The daemon decides the rest, in Rust, on a worker thread: whether to warm (the throttle and the
//! ledger of cmux-tui's `link/start_chat_cli/spare_warm.rs`), the warm or claim call, and the
//! headless Claude login copy onto the warmed or claimed machine (the gate, the remote switch, the
//! machine's reads, the Keychain read and the transfer). The answer is the ledger's, sent as soon
//! as it is written (a claim's at once, a warm's after the platform call); the copy runs after it
//! on its own thread, and reports its end to the same ledger, which a start's `wait-copy` reads.
//!
//! Only the verified app may send it: it carries the person's session and install tokens, which
//! the daemon uses for this one warm and never keeps.

use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;

use super::cloud_conversations::is_app_client;
use crate::Mux;
use crate::cloud_conversations::CloudError;

/// `identify` advertises it once the binary installed a [`SpareWarmService`].
pub const CAPABILITY: &str = "cloud-spare-warm-v1";

/// What the app knows and the daemon cannot read itself: the tokens it holds, and the settings it
/// keeps in its own defaults and cmux.json. Every rule over them is the daemon's.
#[derive(Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct SpareWarmParams {
    /// The platform origin the warm calls (the app's artifacts API base URL).
    pub platform: String,
    /// The cmux API the copy's machine routes and the Release remote switch call (the app's
    /// owner API base URL).
    pub api: String,
    /// The person's Stack session token (the platform's and the machine routes' bearer).
    pub access_token: String,
    /// The install token, whose `team` claim is the backend team the spare is warmed in.
    pub install_token: String,
    /// The signed-in Stack user id.
    pub account: String,
    /// The machine a reserve's or a claim's answer named: no warm, the copy onto it.
    #[serde(default)]
    pub claimed: Option<String>,
    /// The selected Stack team (nil: personal), whose cached login is read first.
    #[serde(default)]
    pub team: Option<String>,
    /// Every scope the person's cached login may sit under: their teams, then personal (null).
    #[serde(default)]
    pub login_scopes: Vec<Option<String>>,
    /// DEV and dogfood builds (`DevTools.isEnabled`).
    #[serde(default)]
    pub dogfood: bool,
    /// `cmux.cloud.passHarnessLogin` from the app's defaults, when set.
    #[serde(default)]
    pub setting: Option<bool>,
    /// The person's kept login choice (`pass`, `machine`, `none`), when they made one.
    #[serde(default)]
    pub stored_choice: Option<String>,
    /// The person's one-time Release consent from cmux.json, when asked.
    #[serde(default)]
    pub consent: Option<bool>,
}

impl std::fmt::Debug for SpareWarmParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpareWarmParams")
            .field("platform", &self.platform)
            .field("api", &self.api)
            .field("access_token", &"[redacted]")
            .field("install_token", &"[redacted]")
            .field("account", &self.account)
            .field("claimed", &self.claimed)
            .field("dogfood", &self.dogfood)
            .finish_non_exhaustive()
    }
}

/// The warm itself, which the binary that has the Cloud transport and the ledger installs.
pub trait SpareWarmService: Send + Sync {
    /// Answers the ledger's answer once it is written, and starts the login copy it hands out (if
    /// any) on its own thread. `still` says whether the account the copy is for is still the
    /// daemon's leased person: false ends the copy as abandoned, so the next warm answers it again.
    fn warm(&self, params: SpareWarmParams, still: Arc<dyn Fn(&str) -> bool + Send + Sync>) -> Value;
}

/// The command, on a cloud request worker thread (`cloud_conversations::is_network`).
pub(super) fn handle(mux: &Mux, client: u64, params: SpareWarmParams) -> anyhow::Result<Value> {
    if !is_app_client(mux, client) {
        return Err(forbidden().into());
    }
    let Some(service) = mux.spare_warm() else {
        return Err(CloudError::Unavailable("this daemon has no Cloud spare warm".into()).into());
    };
    let account = params.account.clone();
    let lease = mux.cloud_conversations().map(|service| service.clone());
    let still: Arc<dyn Fn(&str) -> bool + Send + Sync> = Arc::new(move |copy_account: &str| {
        // The person the app leased last: a switch or a sign-out since ends the copy. A daemon
        // with no person lease (none was ever sent) keeps the app's own account.
        let leased = lease.as_ref().and_then(|lease| lease.lease_person()).map(|(sub, _)| sub);
        copy_account == account && leased.is_none_or(|sub| sub == copy_account)
    });
    Ok(service.warm(params, still))
}

/// The refusal for a connection that is not the verified app.
pub(super) fn forbidden() -> CloudError {
    CloudError::Rejected {
        code: "forbidden".into(),
        message: "cloud-spare-warm requires the verified cmux app connection".into(),
        retryable: false,
    }
}

#[cfg(test)]
#[path = "cloud_spare_warm_tests.rs"]
mod tests;
