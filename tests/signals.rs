//! What `battuta` does to a process that has not agreed to it.
//!
//! **This file holds one test, and it has to stay that way.** Signal
//! dispositions and the consent flag are both process-global, the test harness
//! runs a binary's tests in one process, and consent is one-way — so a second
//! test in this file that consented would decide the outcome of this one by
//! winning a race. Every test of the *consenting* side belongs in `play.rs`,
//! where the consumer is `mid` in a process of its own.

mod common;

use common::FIXTURE;
use std::ptr;

/// A handler that does nothing, existing only to be found again afterwards.
extern "C" fn a_consumers_own_handler(_signal: libc::c_int) {}

/// A library consumer keeps the signal handlers it installed.
///
/// Auditioning a passage writes a temporary Take, and removing that on a signal
/// means `SIGINT`, `SIGTERM` and `SIGHUP` handlers — a process-global resource
/// this library used to take on first use, permanently, without being asked. A
/// host application holding its own `SIGTERM` handler for its own shutdown lost
/// it the first time it played four Bars, and afterwards died where it used to
/// shut down cleanly.
///
/// So the audition below is a real one — a passage cut, written and handed to a
/// synthesiser — by a consumer that never consented. ADR-0010.
#[test]
fn a_consumer_that_does_not_consent_keeps_its_own_signal_handlers() {
    let caught = [libc::SIGINT, libc::SIGTERM, libc::SIGHUP];
    let ours = a_consumers_own_handler as *const () as libc::sighandler_t;

    for signal in caught {
        let mut installing: libc::sigaction = unsafe { std::mem::zeroed() };
        installing.sa_sigaction = ours;
        // SAFETY: installing a handler that does nothing, on a signal this test
        // never raises.
        unsafe {
            libc::sigemptyset(&mut installing.sa_mask);
            libc::sigaction(signal, &installing, ptr::null_mut());
        }
    }

    // A real audition, through the library rather than through `mid`: the
    // passage is cut, written somewhere temporary, and handed over. Nothing here
    // calls `remove_temporary_takes_on_signals`.
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = common::fake_fluidsynth(dir.path());
    let soundfont = common::fake_soundfont(&dir.path().join("chosen.sf2"));
    std::env::set_var("PATH", &fake.dir);
    std::env::remove_var(battuta::rig::SOUNDFONT_ENV);

    let bars: battuta::BarRange = "5:8".parse().expect("a Bar range");
    let audition = battuta::rig::play(
        std::path::Path::new(FIXTURE),
        Some(bars),
        Some(soundfont),
        &mut |_rig| {},
    )
    .expect("the passage is auditioned");
    // Without this the test would pass on an audition that never happened, and
    // so on handlers that were never in danger.
    assert_eq!(audition.bars, Some(bars), "no passage was heard");
    assert!(fake.handed.exists(), "the synthesiser was never reached");

    for signal in caught {
        let mut found: libc::sigaction = unsafe { std::mem::zeroed() };
        // SAFETY: reading a disposition, changing nothing.
        unsafe { libc::sigaction(signal, ptr::null(), &mut found) };
        assert_eq!(
            found.sa_sigaction, ours,
            "battuta took signal {signal} from a consumer that never offered it"
        );
    }
}
