//! Cloud backup of the notes, to Sicompass Cloud.
//!
//! Off until the user ticks "enable cloud backup" in the notes settings. Then:
//!
//! - **Where the user stands** comes from the host (`license.standing` for
//!   Sicompass Cloud), which verified the certificate. The row at the top of the
//!   notes says it, in the user's language, and never links anywhere: buying
//!   and redeeming are in store, tiers.
//! - **Uploads** wait until the notes have been quiet for a few seconds
//!   (`sicompass_payments::debounce`), and then run as a background task: a
//!   fresh instance of this plugin reads `/storage`, gets the redeem token from
//!   `license.token` (which the host gives only for this plugin's own service)
//!   and uploads through `net`. Nothing blocks the UI.
//! - **Restore** is a task too, and never runs over notes that exist.
//!
//! The paywall is on the service, never on the data: whatever the standing,
//! the notes are shown and saved to disk. Only the copy on the server is paid.

use sicompass_payments::debounce::Debounce;
use sicompass_payments::protocol::{self, Request, Response};
use sicompass_payments::row::{self, Standing};
use sicompass_payments::snapshot::read_store;
use sicompass_sdk::ffon::FfonElement;
use sicompass_sdk::tags;

use crate::localize;

/// The tier that pays for the service; `plugin.json` names it as `service`.
pub const TIER: &str = "friendlyflow/cloud";
/// The backup server, the only host `plugin.json` allows.
pub const SERVER: &str = "https://store.sicompass.org";
/// The store's name on the server.
pub const PLUGIN: &str = "notes";
/// The settings key of the "enable cloud backup" switch.
pub const ENABLE_KEY: &str = "notesCloudBackup";
/// The `<id>` the backup row carries, so it is never taken for a note (note
/// ids are numbers).
pub const ROW_ID: &str = "cloud";

pub const TASK_BACKUP: &str = "backup";
pub const TASK_RESTORE: &str = "restore";

/// What the cloud needs from the host, so the logic runs natively in tests.
pub trait CloudHost {
    fn now_millis(&self) -> u64;
    fn standing(&self) -> Standing;
    fn spawn(&self, task: &str, input: &[u8]) -> Result<u64, String>;
    /// The redeem token for [`TIER`], which the host gives only because
    /// `plugin.json` names that tier as this plugin's service.
    fn token(&self) -> Option<String>;
}

/// What a finished task changed for the notes.
#[derive(Debug, PartialEq, Eq)]
pub enum Finished {
    Nothing,
    /// The notes on disk were replaced by the server's copy: reload them.
    Restored,
}

#[derive(Debug, Default)]
pub struct Cloud {
    enabled: bool,
    debounce: Debounce,
    /// Hash of the last snapshot the server acknowledged: an unchanged store
    /// costs no upload at all.
    last_hash: Option<String>,
    backup_task: Option<u64>,
    restore_task: Option<u64>,
    error: Option<String>,
    announcement: Option<String>,
    refresh: bool,
}

impl Cloud {
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Switch it on or off. Switched on without a subscription, the reason is
    /// said once, out loud and as an error, rather than silently not working.
    pub fn set_enabled(&mut self, enabled: bool, host: &dyn CloudHost) {
        let was = self.enabled;
        self.enabled = enabled;
        if enabled && !was && !host.standing().backs_up() {
            let line = localize::t("notes-cloud-needs-subscription");
            self.announcement = Some(line.clone());
            self.error = Some(line);
        }
        self.refresh = true;
    }

    /// The switch as saved, at start-up. Nothing is announced: the notice
    /// about a missing subscription is for the moment the user turns it on.
    pub fn restore_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// An upload or a restore is running, so closing the tab would lose it.
    pub fn is_busy(&self) -> bool {
        self.backup_task.is_some() || self.restore_task.is_some()
    }

    /// A setting the host passed on. Returns whether it was ours.
    pub fn on_setting_change(&mut self, key: &str, value: &str, host: &dyn CloudHost) -> bool {
        if key != ENABLE_KEY {
            return false;
        }
        self.set_enabled(value == "true", host);
        true
    }

    /// The row at the top of the notes, while the switch is on.
    pub fn row(&self, host: &dyn CloudHost) -> Option<FfonElement> {
        if !self.enabled {
            return None;
        }
        let (message, days) = row::row_message(host.standing());
        let mut args = localize::Args::new();
        args.set("days", days);
        let text = localize::t_args(&format!("notes-cloud-{message}"), &args);
        Some(FfonElement::new_str(format!(
            "{}{text}",
            tags::format_id(ROW_ID)
        )))
    }

    /// Whether a row is the backup row (rendered, never stored).
    pub fn is_row(raw: &str) -> bool {
        let prefix = raw.split("<input>").next().unwrap_or(raw);
        tags::extract_id(prefix).as_deref() == Some(ROW_ID)
    }

    /// The notes were saved at `now`. Queues only: this runs on every edit.
    pub fn mark_dirty(&mut self, host: &dyn CloudHost) {
        if self.enabled {
            self.debounce.mark(host.now_millis());
        }
    }

    /// Called every frame: start an upload once one is due.
    pub fn tick(&mut self, host: &dyn CloudHost) {
        if !self.enabled || self.backup_task.is_some() || self.restore_task.is_some() {
            return;
        }
        if !self.debounce.take_due(host.now_millis()) {
            return;
        }
        if !host.standing().backs_up() {
            return;
        }
        let input = self.last_hash.clone().unwrap_or_default();
        match host.spawn(TASK_BACKUP, input.as_bytes()) {
            Ok(id) => self.backup_task = Some(id),
            Err(e) => self.fail(e),
        }
    }

    /// Start pulling the server's copy over empty notes.
    pub fn start_restore(&mut self, host: &dyn CloudHost) {
        if self.restore_task.is_some() {
            return;
        }
        match host.spawn(TASK_RESTORE, &[]) {
            Ok(id) => self.restore_task = Some(id),
            Err(e) => self.error = Some(e),
        }
    }

    /// The end of a task this spawned.
    pub fn on_task_done(&mut self, id: u64, result: Result<Vec<u8>, String>) -> Finished {
        self.refresh = true;
        if self.backup_task == Some(id) {
            self.backup_task = None;
            match result {
                Ok(hash) => {
                    if !hash.is_empty() {
                        self.last_hash = Some(String::from_utf8_lossy(&hash).into_owned());
                    }
                }
                Err(e) => self.fail(e),
            }
            return Finished::Nothing;
        }
        if self.restore_task == Some(id) {
            self.restore_task = None;
            let (key, restored) = match result {
                Ok(done) if done == b"restored" => ("notes-restore-done", true),
                Ok(_) => ("notes-restore-empty", false),
                Err(e) if e == protocol::RESTORE_REFUSED => ("notes-restore-refused", false),
                Err(e) => {
                    self.fail(e);
                    return Finished::Nothing;
                }
            };
            self.announcement = Some(localize::t(key));
            return if restored {
                Finished::Restored
            } else {
                Finished::Nothing
            };
        }
        Finished::Nothing
    }

    fn fail(&mut self, reason: String) {
        let mut args = localize::Args::new();
        args.set("reason", reason);
        self.error = Some(localize::t_args("notes-cloud-failed", &args));
    }

    pub fn take_error(&mut self) -> Option<String> {
        self.error.take()
    }

    pub fn take_announcement(&mut self) -> Option<String> {
        self.announcement.take()
    }

    pub fn needs_refresh(&self) -> bool {
        self.refresh
    }

    pub fn clear_needs_refresh(&mut self) {
        self.refresh = false;
    }
}

// ---------------------------------------------------------------------------
// The tasks, run in a fresh instance of the plugin
// ---------------------------------------------------------------------------

/// Upload `/storage` unless it hashes to `last_hash`. Returns the hash the
/// server now holds, or nothing when there was nothing to do.
pub fn run_backup(
    root: &std::path::Path,
    last_hash: &[u8],
    token: Option<String>,
    send: protocol::Send,
) -> Result<Vec<u8>, String> {
    let snapshot = read_store(root, PLUGIN)?;
    if snapshot.hash.as_bytes() == last_hash {
        return Ok(Vec::new());
    }
    let token = token.ok_or("no Sicompass Cloud licence redeemed (store, tiers)")?;
    protocol::put_snapshot(send, SERVER, &token, &snapshot)?;
    Ok(snapshot.hash.into_bytes())
}

/// Restore the server's copy into an empty `/storage`: `restored` or `empty`.
pub fn run_restore(
    root: &std::path::Path,
    token: Option<String>,
    send: protocol::Send,
) -> Result<Vec<u8>, String> {
    let token = token.ok_or("no Sicompass Cloud licence redeemed (store, tiers)")?;
    let restored = protocol::restore(send, SERVER, &token, root, PLUGIN)?;
    Ok(if restored {
        b"restored".to_vec()
    } else {
        b"empty".to_vec()
    })
}

/// `send` over the plugin's `net` interface.
#[cfg(target_arch = "wasm32")]
pub fn net_send(req: &Request) -> Result<Response, String> {
    use sicompass_pdk::net;
    let resp = net::fetch(&net::HttpRequest {
        method: req.method.to_owned(),
        url: req.url.clone(),
        headers: req.headers.clone(),
        body: req.body.clone(),
    })?;
    Ok(Response {
        status: resp.status,
        body: resp.body,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn net_send(_req: &Request) -> Result<Response, String> {
    Err("no network outside the sandbox".to_owned())
}

/// The host, inside the sandbox.
pub struct PluginHost;

#[cfg(target_arch = "wasm32")]
impl CloudHost for PluginHost {
    fn now_millis(&self) -> u64 {
        sicompass_pdk::host::now_millis()
    }

    fn standing(&self) -> Standing {
        use sicompass_pdk::license::{self, TierStatus};
        let s = license::standing(TIER);
        match s.status {
            TierStatus::Active => Standing::Active {
                renews_in_days: s.days,
            },
            TierStatus::Grace => Standing::Grace { days_left: s.days },
            TierStatus::Expired => Standing::Expired { days_ago: s.days },
            TierStatus::Missing => Standing::Missing,
        }
    }

    fn spawn(&self, task: &str, input: &[u8]) -> Result<u64, String> {
        sicompass_pdk::tasks::spawn(task, input)
    }

    fn token(&self) -> Option<String> {
        sicompass_pdk::license::token(TIER)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl CloudHost for PluginHost {
    fn now_millis(&self) -> u64 {
        0
    }

    fn standing(&self) -> Standing {
        Standing::Missing
    }

    fn spawn(&self, _task: &str, _input: &[u8]) -> Result<u64, String> {
        Err("no tasks outside the sandbox".to_owned())
    }

    fn token(&self) -> Option<String> {
        None
    }
}
