use super::*;
use std::cell::{Cell, RefCell};

const TOKEN: &[u8] = b"sk-ant-oat01-0123456789abcdefghijklmnopqrstuvwxyz";

/// A machine and a Keychain in memory: `guest` is what the kind probe prints, `login` the
/// machine login probe's word, `cached` the Keychain's login, `switch` the Release switch.
struct Fake {
    guest: RefCell<Vec<&'static str>>,
    login: &'static str,
    cached: Result<Option<&'static [u8]>, ()>,
    switch: Option<bool>,
    preflight: i64,
    install: i64,
    execs: RefCell<Vec<String>>,
    writes: RefCell<Vec<(String, Vec<u8>)>>,
    clock: Cell<Duration>,
    start: Instant,
}

impl Fake {
    fn new() -> Self {
        Self {
            guest: RefCell::new(vec!["vm-kind:owner\n"]),
            login: "none",
            cached: Ok(Some(TOKEN)),
            switch: Some(true),
            preflight: 0,
            install: 0,
            execs: RefCell::default(),
            writes: RefCell::default(),
            clock: Cell::new(Duration::ZERO),
            start: Instant::now(),
        }
    }

    fn ran(&self, needle: &str) -> usize {
        self.execs.borrow().iter().filter(|command| command.contains(needle)).count()
    }
}

impl Effects for Fake {
    fn remote_switch(&self) -> Option<bool> {
        self.switch
    }

    fn exec(&self, command: &str, _timeout_ms: u64) -> Option<(i64, String)> {
        self.execs.borrow_mut().push(command.to_owned());
        if command.contains("vm-kind:%s") {
            let mut guest = self.guest.borrow_mut();
            let next = if guest.len() > 1 { guest.remove(0) } else { guest[0] };
            return Some((0, next.to_owned()));
        }
        if command.contains("CMUX_LOGIN_PROBE") {
            return Some((0, format!("{}\n", self.login)));
        }
        if command == scripts::PREFLIGHT {
            return Some((self.preflight, String::new()));
        }
        if command.contains("CMUX_CREDENTIAL_INSTALL") {
            return Some((self.install, String::new()));
        }
        Some((0, String::new()))
    }

    fn write(&self, path: &str, data: &[u8]) -> bool {
        self.writes.borrow_mut().push((path.to_owned(), data.to_vec()));
        true
    }

    fn keychain(&self, _scope: Option<&str>) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        self.cached.map(|token| token.map(|token| Zeroizing::new(token.to_vec())))
    }

    fn pause(&self, duration: Duration) {
        self.clock.set(self.clock.get() + duration);
    }

    fn now(&self) -> Instant {
        self.start + self.clock.get()
    }
}

fn params(dogfood: bool) -> SpareWarmParams {
    SpareWarmParams {
        platform: "https://platform.example".into(),
        api: "https://api.example".into(),
        access_token: "access".into(),
        install_token: "install".into(),
        account: "user_a".into(),
        dogfood,
        consent: Some(true),
        ..SpareWarmParams::default()
    }
}

#[test]
fn a_dogfood_warm_installs_the_cached_login_on_an_owner_machine() {
    let fake = Fake::new();
    let (installed, summary) = copy(&params(true), &|| true, &fake);
    assert!(installed, "{summary}");
    assert!(summary.ends_with("-> installed"), "{summary}");
    let writes = fake.writes.borrow();
    assert_eq!(writes.len(), 1);
    assert!(writes[0].0.starts_with("/home/cmux/.cmux/credential-transfer/"));
    assert_eq!(writes[0].1, TOKEN);
    assert_eq!(fake.ran("CMUX_CREDENTIAL_INSTALL"), 1);
    // The token goes only in the write's body, never in a command.
    assert!(fake.execs.borrow().iter().all(|command| !command.contains("sk-ant-")));
}

#[test]
fn a_machine_with_a_working_login_keeps_it() {
    let fake = Fake { login: "working", ..Fake::new() };
    let (installed, summary) = copy(&params(true), &|| true, &fake);
    assert!(!installed, "{summary}");
    assert!(fake.writes.borrow().is_empty());
}

#[test]
fn a_team_guest_never_gets_a_copy_in_release() {
    let fake = Fake { guest: RefCell::new(vec!["vm-kind:team\n"]), ..Fake::new() };
    let (installed, summary) = copy(&params(false), &|| true, &fake);
    assert!(!installed, "{summary}");
    assert!(fake.writes.borrow().is_empty());
    assert_eq!(fake.ran("CMUX_LOGIN_PROBE"), 0, "a guest that takes no login is not probed");
}

#[test]
fn release_copies_nothing_while_the_remote_switch_is_off() {
    let fake = Fake { switch: Some(false), ..Fake::new() };
    let (installed, summary) = copy(&params(false), &|| true, &fake);
    assert!(!installed, "{summary}");
    assert!(fake.execs.borrow().is_empty(), "nothing is read on the machine");
}

#[test]
fn release_without_consent_refuses_headless() {
    let fake = Fake::new();
    let params = SpareWarmParams { consent: None, ..params(false) };
    let (installed, _) = copy(&params, &|| true, &fake);
    assert!(!installed);
    assert!(fake.writes.borrow().is_empty());
}

#[test]
fn no_cached_login_and_an_unreadable_keychain_say_so() {
    let fake = Fake { cached: Ok(None), ..Fake::new() };
    let (installed, summary) = copy(&params(true), &|| true, &fake);
    assert!(!installed && fake.writes.borrow().is_empty(), "{summary}");
    let fake = Fake { cached: Err(()), ..Fake::new() };
    let (installed, summary) = copy(&params(true), &|| true, &fake);
    assert!(!installed && fake.writes.borrow().is_empty(), "{summary}");
}

#[test]
fn a_kept_machine_or_none_choice_copies_nothing() {
    for word in ["machine", "none"] {
        let fake = Fake::new();
        let params = SpareWarmParams { stored_choice: Some(word.into()), ..params(true) };
        let (installed, summary) = copy(&params, &|| true, &fake);
        assert!(!installed, "{word}: {summary}");
        assert!(fake.writes.borrow().is_empty(), "{word}");
    }
}

#[test]
fn an_account_switch_stops_the_copy_before_the_write() {
    let fake = Fake::new();
    let calls = Cell::new(0);
    // The first ask (before the machine reads) says yes, every later one no.
    let still = || {
        calls.set(calls.get() + 1);
        calls.get() == 1
    };
    let (installed, _) = copy(&params(true), &still, &fake);
    assert!(!installed);
    assert!(fake.writes.borrow().is_empty());
}

#[test]
fn an_unbound_guest_is_read_again_until_it_binds() {
    let fake = Fake {
        guest: RefCell::new(vec!["vm-kind:\n", "vm-kind:\n", "vm-kind:owner\n"]),
        ..Fake::new()
    };
    let (installed, summary) = copy(&params(true), &|| true, &fake);
    assert!(installed, "{summary}");
    assert_eq!(fake.ran("vm-kind:%s"), 3);
}

#[test]
fn a_failed_install_removes_the_staged_copy() {
    let fake = Fake { install: 72, ..Fake::new() };
    let (installed, _) = copy(&params(true), &|| true, &fake);
    assert!(!installed);
    let path = fake.writes.borrow()[0].0.clone();
    assert_eq!(fake.ran(&format!("rm -f -- '{path}'")), 1);
}

#[test]
fn the_keychain_account_is_the_swift_transfer_caches() {
    // CloudCredentialLogin.name: SHA-256 of "user:team|personal:", 12 bytes, hex.
    assert_eq!(keychain_account("user_a", None), "*/CLAUDE_TRANSFER_3fe7adc33a5c773f6984a2a4");
    assert_eq!(keychain_account("user_a", Some("team_x")), "*/CLAUDE_TRANSFER_7934e45098f99257cb904564");
}

#[test]
fn only_a_setup_token_is_valid_and_the_envelope_is_read() {
    assert!(valid(TOKEN));
    assert!(!valid(b"sk-ant-oat01-short"));
    assert!(!valid(b"sk-ant-api03-0123456789abcdefghijklmnopqrstuvwxyz"));
    assert!(!valid(b"sk-ant-oat01-0123456789abcdefghij klmnopqrstuvwxyz"));
    let line = format!(r#"{{"created": 1, "credential": "{}"}}"#, BASE64.encode(TOKEN));
    assert_eq!(envelope(line.as_bytes()).as_deref().map(Vec::as_slice), Some(TOKEN));
    assert!(envelope(b"not json").is_none());
}
