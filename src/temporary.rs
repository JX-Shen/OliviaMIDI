//! Files written where nothing will find them afterwards.
//!
//! `mid play --bars` hands FluidSynth a passage cut out of the user's Take, and
//! `mid render` has FluidSynth write a WAV that is not yet the one asked for.
//! Each file is an implementation detail of its command: it is never presented
//! as anybody's file, and it must not exist once the command is over. Not after
//! success, not after a failure, and not after the Ctrl-C that — because `play`
//! blocks for as long as the audio lasts — is the ordinary way to stop it part
//! way through, rather than an edge case.
//!
//! Drop covers the first two. The third needs a signal handler; a signal
//! handler may only reach a global; so this module owns both, and they are here
//! rather than in `rig` because what must not be left behind is a fact about
//! the file, not about playback.
//!
//! Unix only, like everything else in this project that touches a process.

use crate::error::{Error, Result};
use crate::take::Take;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::Once;

/// A file on disk that removes itself: when this value drops, and when the
/// process is killed by a signal it is allowed to catch.
pub(crate) struct TemporaryFile {
    path: PathBuf,
    /// Where this path sits in `TO_REMOVE`, when it found room there.
    slot: Option<usize>,
}

impl TemporaryFile {
    /// Take responsibility for the going of a file that `tempfile` created.
    ///
    /// Registered before anything is written to it: the file exists from the
    /// moment it is created, so that is the moment it becomes something to
    /// clean up. Removal becomes this type's own job rather than
    /// `NamedTempFile`'s, so that one owner decides when it happens and in what
    /// order.
    fn owning(file: tempfile::NamedTempFile) -> std::io::Result<TemporaryFile> {
        let (_, path) = file.keep().map_err(|failed| failed.error)?;
        Ok(TemporaryFile {
            slot: remember(&path),
            path,
        })
    }

    /// An empty file beside `destination`, for something to be written into
    /// and then renamed onto it.
    ///
    /// Beside it, because a rename across two filesystems is not a rename and
    /// not atomic. Named so that one left behind by a process that could not
    /// catch its death does not read as somebody's file.
    pub(crate) fn beside(destination: &Path, suffix: &str) -> std::io::Result<TemporaryFile> {
        install();
        let directory = destination
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let file = tempfile::Builder::new()
            .prefix(".battuta-writing-")
            .suffix(suffix)
            .tempfile_in(directory)?;
        TemporaryFile::owning(file)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Rename this file onto `destination`, which then holds either what it
    /// held before or the whole of this.
    ///
    /// The registration outlives the rename by the length of `Drop`. A signal
    /// landing in between unlinks a name that no longer exists, which is
    /// harmless; deregistering first would leave a window where it is neither.
    pub(crate) fn persist(self, destination: &Path) -> std::io::Result<()> {
        std::fs::rename(&self.path, destination)
    }
}

/// A Take written where nothing will find it afterwards.
pub(crate) struct TemporaryTake {
    file: TemporaryFile,
}

impl TemporaryTake {
    /// Write a Take somewhere temporary and take responsibility for its going.
    pub(crate) fn holding(take: &Take) -> Result<TemporaryTake> {
        install();
        let file = tempfile::Builder::new()
            .prefix("battuta-passage-")
            .suffix(".mid")
            .tempfile()
            .map_err(Error::PassageUnwritable)?;
        let temporary = TemporaryTake {
            file: TemporaryFile::owning(file).map_err(Error::PassageUnwritable)?,
        };
        // Written *into* the registered file, which is the one place in this
        // crate that must not use `Take::write`. That writes a new file beside
        // the destination and renames it on, which is right for a Take somebody
        // keeps and wrong here: this path is on the handler's list and the
        // intermediate file would not be, so a signal landing mid-write would
        // remove the registered path and leave the other behind.
        //
        // Creating the file and filling it are one condition with one remedy,
        // and the user is never told this file's name: it is not a file they
        // have, so it is not a file they can go and fix.
        std::fs::write(temporary.path(), take.bytes()).map_err(Error::PassageUnwritable)?;
        Ok(temporary)
    }

    pub(crate) fn path(&self) -> &Path {
        self.file.path()
    }
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        // The file goes first and the registration second, never the other way
        // round. A signal landing between the two must find the path either
        // already gone or still on the list — never neither, which is the one
        // ordering that leaves the file behind.
        let _ = std::fs::remove_file(&self.path);
        if let Some(slot) = self.slot {
            let path = TO_REMOVE[slot].swap(ptr::null_mut(), Ordering::AcqRel);
            if !path.is_null() {
                // SAFETY: this is the `CString` `remember` leaked into the
                // slot, and clearing the slot first means no handler can be
                // looking at it any more.
                unsafe { drop(CString::from_raw(path)) };
            }
        }
    }
}

/// How many temporary files may be outstanding at once.
///
/// `mid play` holds one at most, and `mid render` two. The room for eight is for a library consumer
/// auditioning passages on several threads; one that wanted more would still
/// get its files removed on the way out, just not by a signal. The number is
/// part of what #4 promises rather than a detail behind it, which is why
/// growing it is not the answer to a consumer that needs more.
const SLOTS: usize = 8;

/// The paths a signal handler has to remove.
///
/// A fixed array of raw pointers, because a handler may not allocate, may not
/// take a lock, and may not read a `Vec` another thread might be growing.
static TO_REMOVE: [AtomicPtr<libc::c_char>; SLOTS] =
    [const { AtomicPtr::new(ptr::null_mut()) }; SLOTS];

/// Put a path where the handler can see it, if there is room.
fn remember(path: &Path) -> Option<usize> {
    let text = CString::new(path.as_os_str().as_bytes()).ok()?;
    let raw = text.into_raw();
    for (index, slot) in TO_REMOVE.iter().enumerate() {
        if slot
            .compare_exchange(ptr::null_mut(), raw, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            return Some(index);
        }
    }
    // SAFETY: nothing else ever saw this pointer, so taking it back is safe.
    unsafe { drop(CString::from_raw(raw)) };
    None
}

/// Remove what is registered, then die of the signal rather than of this.
///
/// Re-raising with the default disposition restored is what makes a shell say
/// the command was interrupted, instead of reporting some exit code invented
/// here. The signal is blocked for the length of this handler, so the raise
/// lands the moment it returns.
extern "C" fn remove_and_die(signal: libc::c_int) {
    for slot in &TO_REMOVE {
        let path = slot.swap(ptr::null_mut(), Ordering::AcqRel);
        if !path.is_null() {
            // SAFETY: `unlink` is async-signal-safe, and the pointer is a
            // leaked NUL-terminated string that nothing frees while it is here.
            unsafe { libc::unlink(path) };
        }
    }
    // SAFETY: both calls are async-signal-safe.
    unsafe {
        libc::signal(signal, libc::SIG_DFL);
        libc::raise(signal);
    }
}

static INSTALLED: Once = Once::new();

/// Whether the process has agreed to have its signals caught. Nothing is caught
/// until it has.
static CONSENTED: AtomicBool = AtomicBool::new(false);

/// Consent to `battuta` catching the signals that mean *stop*.
///
/// A temporary Take is removed on every route out of a command that runs a
/// destructor. A signal is the one route that does not, and covering it means
/// installing handlers for `SIGINT`, `SIGTERM` and `SIGHUP`. Those are a
/// *process-global* resource, and a library has no business taking one without
/// being asked: a host application holding its own `SIGTERM` handler for its own
/// shutdown would lose it — permanently, because nothing here restores it — the
/// first time it auditioned a single passage, and would then die where it used
/// to shut down cleanly.
///
/// So it is asked for. `mid` consents in `main`, because `mid` owns its process.
/// A consumer that does not consent keeps its own handlers and still has every
/// temporary Take removed by `Drop`: on success, on failure, on an unwinding
/// panic. What it gives up is the signal alone, and only for the seconds an
/// audition is actually running.
///
/// Calling this more than once, or from several threads, is harmless. Handlers
/// are still installed lazily — at the first temporary Take rather than here —
/// so a process that consents and then never auditions anything has its signals
/// left alone as well. See ADR-0006.
pub fn remove_temporary_takes_on_signals() {
    CONSENTED.store(true, Ordering::Release);
}

/// Catch the three signals that mean *stop*, the first time there is anything
/// to clean up in a process that has agreed to it.
///
/// Lazily, so that a consumer of this library which never writes a temporary
/// Take never has its signals touched. A signal that is already being ignored
/// is left alone: that is a decision the process made before `battuta` was
/// called — usually because a shell put it in the background — and taking it
/// back would kill a job that asked not to be killed.
///
/// `SIGQUIT` is deliberately not caught. Its whole purpose is an abrupt abort
/// with a core dump, and a file left in the temporary directory is a smaller
/// harm than interfering with somebody debugging.
fn install() {
    // Read before the `Once` and not inside it, so that consent arriving after
    // the first temporary Take still installs the handlers at the next one.
    if !CONSENTED.load(Ordering::Acquire) {
        return;
    }
    INSTALLED.call_once(|| {
        for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            // SAFETY: `remove_and_die` calls only async-signal-safe functions.
            unsafe {
                let mut current: libc::sigaction = std::mem::zeroed();
                libc::sigaction(signal, ptr::null(), &mut current);
                if current.sa_sigaction == libc::SIG_IGN {
                    continue;
                }
                let mut ours: libc::sigaction = std::mem::zeroed();
                ours.sa_sigaction = remove_and_die as *const () as libc::sighandler_t;
                libc::sigemptyset(&mut ours.sa_mask);
                libc::sigaction(signal, &ours, ptr::null_mut());
            }
        }
    });
}
