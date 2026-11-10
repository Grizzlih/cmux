//! The daemon's `cloud-spare-warm` (cmux-tui-core `server/cloud_spare_warm.rs`): the App's Cloud
//! intent runs the same ledger and platform call as `cmux link start-chat warm-spare`, in the
//! daemon process, and the login copy it hands out runs here too ([`super::login_copy`]), so the
//! App spawns no process and its main actor waits on nothing.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use cmux_tui_core::Mux;
use cmux_tui_core::server::cloud_spare_warm::{ReservationParams, SpareWarmParams, SpareWarmService};
use serde_json::Value;

use super::{Request, answer, login_copy, now_ms, post};

/// The installed service: the start-chat state folder is read on every call, as the CLI does.
struct DaemonSpareWarm;

/// Installs `cloud-spare-warm-v1` on the daemon's mux.
pub(crate) fn install(mux: &Arc<Mux>) {
    if !mux.install_spare_warm(Arc::new(DaemonSpareWarm)) {
        crate::client_log::stderr_log!("startup", "cloud spare warm: already installed");
    }
}

impl SpareWarmService for DaemonSpareWarm {
    fn reservation(&self, params: ReservationParams) -> Value {
        super::super::reservations::daemon(params.request, params.release, &params.access_token, &params.install_token)
    }

    fn warm(&self, params: SpareWarmParams, still: Arc<dyn Fn(&str) -> bool + Send + Sync>) -> Value {
        let Some(ledger) = super::super::ledger::path() else {
            return super::refused("ledger_failed", "this Mac has no state folder for the spare warm");
        };
        let state = ledger.parent().map_or_else(|| ledger.clone(), Path::to_path_buf);
        warm(params, &state, still, &post, &|params, state, still, machine, team| {
            std::thread::Builder::new()
                .name("cloud-warm-login-copy".into())
                .spawn(move || copy_then_report(&params, &state, &*still, &machine, &team))
                .is_ok()
        })
    }
}

/// Starts the copy the answer hands out on its own thread; false when it could not start.
type Spawn<'a> = &'a dyn Fn(
    SpareWarmParams,
    PathBuf,
    Arc<dyn Fn(&str) -> bool + Send + Sync>,
    String,
    String,
) -> bool;

/// The ledger's answer to `params` (a warm, or the claim of `params.claimed`), and the copy it
/// hands out started with `spawn`. A copy that cannot start is reported abandoned at once, so the
/// next warm answers it again and no start waits on it.
fn warm(
    params: SpareWarmParams,
    state: &Path,
    still: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    transport: super::Transport<'_>,
    spawn: Spawn<'_>,
) -> Value {
    let request = Request {
        platform: Some(params.platform.clone()),
        account: params.account.clone(),
        abandoned: None,
        finished: None,
        wait_copy: None,
        claimed: params.claimed.clone(),
    };
    let reply = answer(request, &params.access_token, &params.install_token, state, now_ms(), transport);
    let copy = reply.get("copy").and_then(|copy| {
        let machine = copy.get("machine")?.as_str()?.to_owned();
        let team = copy.get("team")?.as_str()?.to_owned();
        (copy.get("account")?.as_str()? == params.account).then_some((machine, team))
    });
    if let Some((machine, team)) = copy {
        let report = params.clone();
        if !spawn(params, state.to_path_buf(), still, machine.clone(), team) {
            end(&report, state, &machine, false, transport);
        }
    }
    reply
}

/// The copy, then its end to the ledger: finished when it installed or ended for good, abandoned
/// when the person changed meanwhile (the next warm answers it again).
fn copy_then_report(
    params: &SpareWarmParams,
    state: &Path,
    still: &(dyn Fn(&str) -> bool + Send + Sync),
    machine: &str,
    team: &str,
) {
    let still_person = || still(&params.account);
    let installed = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(runtime) => {
            let live = login_copy::Live { params, machine, team, runtime };
            let (installed, summary) = login_copy::copy(params, &still_person, &live);
            crate::client_log::stderr_log!(
                "cloud",
                "{}: harness login {summary}",
                login_copy::STEP
            );
            installed
        }
        Err(_) => false,
    };
    let kept = installed || still_person();
    end(params, state, machine, kept, &post);
}

/// Reports the copy onto `machine` ended (`kept`) or abandoned to the ledger.
fn end(params: &SpareWarmParams, state: &Path, machine: &str, kept: bool, transport: super::Transport<'_>) {
    let request = Request {
        platform: Some(params.platform.clone()),
        account: params.account.clone(),
        abandoned: (!kept).then(|| machine.to_owned()),
        finished: kept.then(|| machine.to_owned()),
        wait_copy: None,
        claimed: None,
    };
    let _ = answer(request, &params.access_token, &params.install_token, state, now_ms(), transport);
}

#[cfg(test)]
#[path = "daemon_tests.rs"]
mod tests;
