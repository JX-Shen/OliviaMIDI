//! `mid render`, watched through a fake FluidSynth (#70).
//!
//! The fake writes a WAV the test built, so every byte of the expected file is
//! known here: what FluidSynth wrote is the test's own literal, and what the
//! attribution must say is computed from the fixture and the soundfont with
//! `sha2`, never by asking `battuta`. That FluidSynth itself renders one Take
//! byte-identically twice is FluidSynth's, and is recorded by probe on #70.

mod common;

use common::{mid, FAKE_FLUIDSYNTH_VERSION, FIXTURE};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// A small RIFF WAVE file standing in for FluidSynth's output: a `fmt ` chunk
/// for 16-bit stereo at 44.1 kHz and a `data` chunk of `samples` bytes.
fn wave(samples: &[u8]) -> Vec<u8> {
    let mut fmt = Vec::new();
    fmt.extend(1u16.to_le_bytes()); // PCM
    fmt.extend(2u16.to_le_bytes()); // channels
    fmt.extend(44_100u32.to_le_bytes());
    fmt.extend((44_100u32 * 4).to_le_bytes());
    fmt.extend(4u16.to_le_bytes());
    fmt.extend(16u16.to_le_bytes());
    let mut body = b"WAVE".to_vec();
    body.extend(b"fmt ");
    body.extend((fmt.len() as u32).to_le_bytes());
    body.extend(fmt);
    body.extend(b"data");
    body.extend((samples.len() as u32).to_le_bytes());
    body.extend(samples);
    let mut file = b"RIFF".to_vec();
    file.extend((body.len() as u32).to_le_bytes());
    file.extend(body);
    file
}

/// The chunks of a RIFF file, in order, as (id, payload). Panics on a file
/// whose sizes do not add up, which is a failure of the file under test.
fn chunks(bytes: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let riff = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    assert_eq!(
        riff,
        bytes.len() - 8,
        "the RIFF size does not count the file"
    );
    let mut found = Vec::new();
    let mut at = 12;
    while at < bytes.len() {
        let id: [u8; 4] = bytes[at..at + 4].try_into().unwrap();
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        found.push((id, bytes[at + 8..at + 8 + size].to_vec()));
        at += 8 + size + size % 2;
    }
    assert_eq!(at, bytes.len(), "the chunks do not end where the file does");
    found
}

/// The text of one `INFO` field of a rendered file, without its terminator.
fn info(bytes: &[u8], field: &[u8; 4]) -> Option<String> {
    let (_, list) = chunks(bytes)
        .into_iter()
        .find(|(id, payload)| id == b"LIST" && payload.starts_with(b"INFO"))?;
    let mut at = 4;
    while at < list.len() {
        let id = &list[at..at + 4];
        let size = u32::from_le_bytes(list[at + 4..at + 8].try_into().unwrap()) as usize;
        let text = &list[at + 8..at + 8 + size];
        if id == field {
            let text = text
                .strip_suffix(b"\0")
                .expect("INFO text is NUL-terminated");
            return Some(String::from_utf8(text.to_vec()).expect("INFO text is UTF-8"));
        }
        at += 8 + size + size % 2;
    }
    None
}

fn sha256(path: &Path) -> String {
    format!(
        "{:x}",
        Sha256::digest(std::fs::read(path).expect("readable"))
    )
}

/// What the attribution of a render of `take` through `soundfont` must say.
fn expected_attribution(take: &Path, bars: &str, soundfont: &Path) -> String {
    format!(
        "rendered by battuta {}\n\
         take sha256:{}\n\
         bars {bars}\n\
         soundfont {} sha256:{}\n\
         fluidsynth {FAKE_FLUIDSYNTH_VERSION}",
        env!("CARGO_PKG_VERSION"),
        sha256(take),
        soundfont.file_name().unwrap().to_string_lossy(),
        sha256(soundfont),
    )
}

/// One render watched by a fake FluidSynth.
struct Render {
    output: std::process::Output,
    /// What the synthesiser received; empty when it was never reached.
    argv: Vec<String>,
    handed: PathBuf,
    /// The fake's own output, as it wrote it.
    written: Vec<u8>,
    soundfont: PathBuf,
    wav: PathBuf,
}

impl Render {
    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).into_owned()
    }

    fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).into_owned()
    }

    fn rendered(&self) -> Vec<u8> {
        std::fs::read(&self.wav).expect("the rendered file exists")
    }
}

/// The fake's samples: odd in length, so the attribution has to start on a pad.
const SAMPLES: &[u8] = &[1, 2, 3, 4, 5, 6, 7];

fn render(dir: &Path, args: &[&str]) -> Render {
    let soundfont = common::fake_soundfont(&dir.join("chosen.sf2"));
    render_through(dir, args, Some(&soundfont), common::fake_fluidsynth(dir))
}

fn render_through(
    dir: &Path,
    args: &[&str],
    soundfont: Option<&Path>,
    fake: common::FakeFluidsynth,
) -> Render {
    render_writing(dir, args, soundfont, fake, wave(SAMPLES))
}

/// The same, with the fake writing `written` when asked to render.
fn render_writing(
    dir: &Path,
    args: &[&str],
    soundfont: Option<&Path>,
    fake: common::FakeFluidsynth,
    written: Vec<u8>,
) -> Render {
    std::fs::write(&fake.renders, &written).expect("scratch file is writable");
    let out = dir.join("out");
    std::fs::create_dir_all(&out).expect("scratch dir is writable");
    let wav = out.join("take.wav");
    let mut command = mid();
    command.arg("render").args(args).arg("-o").arg(&wav);
    if let Some(soundfont) = soundfont {
        command.arg("--rig").arg(soundfont);
    }
    let output = command.env("PATH", &fake.dir).output().expect("mid runs");
    Render {
        argv: std::fs::read_to_string(&fake.log)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect(),
        handed: fake.handed,
        written,
        soundfont: soundfont.unwrap_or(Path::new("")).to_path_buf(),
        output,
        wav,
    }
}

/// The names of everything in a directory, sorted.
fn listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("the directory is readable")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Exactly what `play` hands FluidSynth, and the two things writing a file
/// needs: where, and as what. The file is a temporary one beside the output,
/// so that the output is renamed into place rather than written through.
#[test]
fn hands_fluidsynth_what_play_hands_it_and_the_file_to_write() {
    let dir = tempfile::tempdir().expect("temp dir");
    let rendered = render(dir.path(), &[FIXTURE]);
    assert!(rendered.output.status.success(), "{}", rendered.stderr());

    let soundfont = rendered.soundfont.to_str().unwrap();
    let wav = PathBuf::from(&rendered.argv[4]);
    assert_eq!(
        rendered.argv,
        vec![
            "-i",
            "-n",
            "-q",
            "-F",
            &rendered.argv[4],
            "-T",
            "wav",
            soundfont,
            FIXTURE
        ]
    );
    assert_eq!(
        wav.parent(),
        rendered.wav.parent(),
        "not written beside its output"
    );
    assert_ne!(
        wav, rendered.wav,
        "FluidSynth wrote through the output itself"
    );
    assert!(!wav.exists(), "the temporary file outlived the command");
}

/// The file holds every byte FluidSynth wrote, where FluidSynth wrote it, and
/// after them the attribution: in `ICMT`, naming the Take and the soundfont by
/// digest, the passage, and the versions.
#[test]
fn the_file_is_what_fluidsynth_wrote_with_the_attribution_after_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let rendered = render(dir.path(), &[FIXTURE]);
    assert!(rendered.output.status.success(), "{}", rendered.stderr());

    let file = rendered.rendered();
    assert_eq!(
        file[8..rendered.written.len()],
        rendered.written[8..],
        "what FluidSynth wrote was not kept as it wrote it"
    );
    let ids: Vec<[u8; 4]> = chunks(&file).into_iter().map(|(id, _)| id).collect();
    assert_eq!(ids, vec![*b"fmt ", *b"data", *b"LIST"]);
    assert_eq!(
        info(&file, b"ICMT").expect("an ICMT field"),
        expected_attribution(Path::new(FIXTURE), "all", &rendered.soundfont)
    );
    assert_eq!(
        info(&file, b"ISFT").expect("an ISFT field"),
        format!("battuta {}", env!("CARGO_PKG_VERSION"))
    );
}

/// A passage is rendered as `play` plays one — the fixture's Bars 5-8 moved to
/// the front — and the attribution says which passage it was.
#[test]
fn a_passage_is_handed_over_as_play_hands_it_and_named_in_the_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let rendered = render(dir.path(), &[FIXTURE, "--bars", "5:8"]);
    assert!(rendered.output.status.success(), "{}", rendered.stderr());

    let passage: Vec<u64> = common::notes(&common::inspect_json(Path::new(FIXTURE)))
        .iter()
        .map(|note| note["start"].as_u64().unwrap())
        .filter(|start| (5760..11520).contains(start))
        .map(|start| start - 5760)
        .collect();
    let handed: Vec<u64> = common::notes(&common::inspect_json(&rendered.handed))
        .iter()
        .map(|note| note["start"].as_u64().unwrap())
        .collect();
    assert_eq!(handed, passage);
    assert!(
        !Path::new(rendered.argv.last().unwrap()).exists(),
        "the temporary passage outlived the command"
    );
    assert_eq!(
        info(&rendered.rendered(), b"ICMT").unwrap(),
        expected_attribution(Path::new(FIXTURE), "5:8", &rendered.soundfont)
    );
}

/// What `mid` controls is the same both times: the arguments, and the file it
/// makes of what FluidSynth wrote. No timestamp, nothing random.
#[test]
fn rendering_one_take_twice_writes_one_file_twice() {
    let first = tempfile::tempdir().expect("temp dir");
    let second = tempfile::tempdir().expect("temp dir");
    let soundfont = common::fake_soundfont(&first.path().join("chosen.sf2"));
    let a = render_through(
        first.path(),
        &[FIXTURE],
        Some(&soundfont),
        common::fake_fluidsynth(first.path()),
    );
    let b = render_through(
        second.path(),
        &[FIXTURE],
        Some(&soundfont),
        common::fake_fluidsynth(second.path()),
    );
    assert!(a.output.status.success() && b.output.status.success());
    let without_the_temporary_name = |argv: &[String]| {
        let mut argv = argv.to_vec();
        argv.remove(4);
        argv
    };
    assert_eq!(
        without_the_temporary_name(&a.argv),
        without_the_temporary_name(&b.argv)
    );
    assert_eq!(a.rendered(), b.rendered());
}

/// The Rig is stated as `play` states it, and the file is said to be unheard.
/// Stdout carries the output's path and nothing else.
#[test]
fn states_the_rig_and_that_nobody_has_heard_it_on_stderr() {
    let dir = tempfile::tempdir().expect("temp dir");
    let rendered = render(dir.path(), &[FIXTURE]);
    assert!(rendered.output.status.success());

    let stderr = rendered.stderr();
    assert!(
        stderr.contains(&format!("rig: {}", rendered.soundfont.display())),
        "{stderr}"
    );
    assert!(stderr.contains("not heard"), "{stderr}");
    assert_eq!(rendered.stdout(), format!("{}\n", rendered.wav.display()));
}

#[test]
fn the_json_payload_carries_the_rig_and_the_attribution() {
    let dir = tempfile::tempdir().expect("temp dir");
    let rendered = render(dir.path(), &[FIXTURE, "--json"]);
    assert!(rendered.output.status.success());

    let document: serde_json::Value =
        serde_json::from_slice(&rendered.output.stdout).expect("render --json is JSON");
    assert_eq!(document["take"], FIXTURE);
    assert_eq!(document["bars"], serde_json::Value::Null);
    assert_eq!(document["rig"], rendered.soundfont.to_str().unwrap());
    assert_eq!(document["output"], rendered.wav.to_str().unwrap());
    let attribution = &document["attribution"];
    assert_eq!(attribution["battuta"], env!("CARGO_PKG_VERSION"));
    assert_eq!(attribution["take_sha256"], sha256(Path::new(FIXTURE)));
    assert_eq!(attribution["soundfont"], "chosen.sf2");
    assert_eq!(attribution["soundfont_sha256"], sha256(&rendered.soundfont));
    assert_eq!(attribution["fluidsynth"], FAKE_FLUIDSYNTH_VERSION);
}

/// No Rig, no render, in the words `play` uses — and nothing written.
#[test]
fn refuses_to_render_with_no_rig_configured() {
    let dir = tempfile::tempdir().expect("temp dir");
    let rendered = render_through(
        dir.path(),
        &[FIXTURE],
        None,
        common::fake_fluidsynth(dir.path()),
    );
    let played = mid()
        .args(["play", FIXTURE])
        .env("PATH", common::empty_path(dir.path()))
        .output()
        .expect("mid runs");

    assert!(!rendered.output.status.success());
    assert!(
        rendered.argv.is_empty(),
        "fluidsynth was reached without a Rig"
    );
    assert_eq!(rendered.stderr(), String::from_utf8_lossy(&played.stderr));
    assert_eq!(
        listing(rendered.wav.parent().unwrap()),
        Vec::<String>::new()
    );
}

/// Not finding FluidSynth is its own failure, as it is for `play`.
#[test]
fn a_missing_fluidsynth_is_a_different_failure_from_a_missing_rig() {
    let dir = tempfile::tempdir().expect("temp dir");
    let soundfont = common::fake_soundfont(&dir.path().join("chosen.sf2"));
    let wav = dir.path().join("take.wav");

    let output = mid()
        .args(["render", FIXTURE, "-o"])
        .arg(&wav)
        .arg("--rig")
        .arg(&soundfont)
        .env("PATH", common::empty_path(dir.path()))
        .output()
        .expect("mid runs");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("fluidsynth is not on PATH"), "{stderr}");
    assert!(!wav.exists());
}

/// A FluidSynth that does not say which version it is cannot be attributed,
/// so nothing is rendered.
#[test]
fn a_fluidsynth_that_states_no_version_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let bin = dir.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let fake = bin.join("fluidsynth");
    std::fs::write(&fake, "#!/bin/sh\necho 'something else'\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    let soundfont = common::fake_soundfont(&dir.path().join("chosen.sf2"));
    let wav = dir.path().join("take.wav");

    let output = mid()
        .args(["render", FIXTURE, "-o"])
        .arg(&wav)
        .arg("--rig")
        .arg(&soundfont)
        .env("PATH", &bin)
        .output()
        .expect("mid runs");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("something else"), "{stderr}");
    assert!(!wav.exists());
}

/// `-o` naming the Take, or the soundfont, by any of its names is refused
/// before FluidSynth is reached, and the file keeps its bytes.
#[test]
fn never_writes_over_the_take_or_the_soundfont() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = dir.path().join("take.mid");
    std::fs::copy(FIXTURE, &take).unwrap();
    let take_link = dir.path().join("take-link.mid");
    std::fs::hard_link(&take, &take_link).unwrap();
    let soundfont = common::fake_soundfont(&dir.path().join("chosen.sf2"));
    let soundfont_link = dir.path().join("sf-link.wav");
    std::os::unix::fs::symlink(&soundfont, &soundfont_link).unwrap();

    for output in [&take, &take_link, &soundfont, &soundfont_link] {
        let fake = common::fake_fluidsynth(dir.path());
        std::fs::write(&fake.renders, wave(SAMPLES)).unwrap();
        let _ = std::fs::remove_file(&fake.log);
        let before_take = std::fs::read(&take).unwrap();
        let before_soundfont = std::fs::read(&soundfont).unwrap();

        let result = mid()
            .arg("render")
            .arg(&take)
            .arg("-o")
            .arg(output)
            .arg("--rig")
            .arg(&soundfont)
            .env("PATH", &fake.dir)
            .output()
            .expect("mid runs");

        assert!(!result.status.success(), "-o {output:?} was accepted");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("never writes over what it reads"),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!fake.log.exists(), "-o {output:?} reached fluidsynth");
        assert_eq!(std::fs::read(&take).unwrap(), before_take);
        assert_eq!(std::fs::read(&soundfont).unwrap(), before_soundfont);
    }
}

/// A FluidSynth that fails leaves the destination as it was, and nothing
/// temporary beside it.
#[test]
fn a_failed_render_leaves_the_destination_as_it_was() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("out");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("take.wav"), b"what was here before").unwrap();
    let soundfont = common::fake_soundfont(&dir.path().join("chosen.sf2"));

    let rendered = render_through(
        dir.path(),
        &[FIXTURE],
        Some(&soundfont),
        common::failing_fake_fluidsynth(dir.path()),
    );

    assert!(!rendered.output.status.success());
    assert!(
        rendered.stderr().contains("fluidsynth failed"),
        "{}",
        rendered.stderr()
    );
    assert_eq!(rendered.rendered(), b"what was here before");
    assert_eq!(listing(&out), vec!["take.wav"]);
}

/// A FluidSynth that succeeds without writing a WAV has rendered nothing that
/// can be attributed; nothing is renamed onto the destination.
#[test]
fn a_render_that_wrote_no_wav_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("out");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("take.wav"), b"what was here before").unwrap();
    let soundfont = common::fake_soundfont(&dir.path().join("chosen.sf2"));

    let rendered = render_writing(
        dir.path(),
        &[FIXTURE],
        Some(&soundfont),
        common::fake_fluidsynth(dir.path()),
        b"not a wave file".to_vec(),
    );

    assert!(!rendered.output.status.success());
    assert!(
        rendered.stderr().contains("did not write a WAV file"),
        "{}",
        rendered.stderr()
    );
    assert_eq!(rendered.rendered(), b"what was here before");
    assert_eq!(listing(&out), vec!["take.wav"]);
}

/// One parser, one message: a Bar range `render` cannot honour is refused in
/// the words `inspect` uses, with or without a Rig, and nothing is reached or
/// written.
#[test]
fn a_bar_range_that_cannot_be_honoured_is_refused_as_inspect_refuses_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    for range in ["9:9", "0:4", "8:5", "nonsense"] {
        let inspected = mid()
            .args(["inspect", FIXTURE, "--bars", range])
            .output()
            .expect("mid runs");
        let refusal = String::from_utf8_lossy(&inspected.stderr).into_owned();
        let soundfont = common::fake_soundfont(&dir.path().join("chosen.sf2"));

        for rig in [Some(soundfont.as_path()), None] {
            let rendered = render_through(
                dir.path(),
                &[FIXTURE, "--bars", range],
                rig,
                common::fake_fluidsynth(dir.path()),
            );
            assert!(!rendered.output.status.success(), "--bars {range} accepted");
            assert!(
                rendered.argv.is_empty(),
                "--bars {range} reached fluidsynth"
            );
            assert_eq!(rendered.stderr(), refusal, "--bars {range}");
            assert!(!rendered.wav.exists());
            let _ = std::fs::remove_file(dir.path().join("fluidsynth-argv"));
        }
    }
}

/// Interrupted mid-render, `mid` takes both of its temporary files with it —
/// the passage it handed over and the WAV FluidSynth was writing — and leaves
/// the destination unmade.
#[test]
fn an_interrupt_takes_both_temporary_files_with_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = common::lingering_fake_fluidsynth(dir.path());
    std::fs::write(&fake.renders, wave(SAMPLES)).unwrap();
    let soundfont = common::fake_soundfont(&dir.path().join("chosen.sf2"));
    let out = dir.path().join("out");
    std::fs::create_dir_all(&out).unwrap();
    let wav = out.join("take.wav");

    let mut child = std::process::Command::new(assert_cmd::cargo::cargo_bin("mid"))
        .args(["render", FIXTURE, "--bars", "5:8", "-o"])
        .arg(&wav)
        .arg("--rig")
        .arg(&soundfont)
        .env("PATH", &fake.dir)
        .env_remove(battuta::rig::SOUNDFONT_ENV)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("mid runs");

    let argv = (0..600)
        .find_map(|_| {
            let lines: Vec<String> = std::fs::read_to_string(&fake.log)
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect();
            if lines.len() == 9 {
                return Some(lines);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            None
        })
        .expect("the fake never recorded what it was given");
    let writing = PathBuf::from(&argv[4]);
    let passage = PathBuf::from(argv.last().unwrap());
    assert!(
        writing.exists() && passage.exists(),
        "nothing was in flight"
    );

    // SAFETY: `kill` on a child this test spawned and has not yet reaped.
    unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGINT) };
    let status = child.wait().expect("mid exits");

    use std::os::unix::process::ExitStatusExt;
    assert_eq!(status.signal(), Some(libc::SIGINT));
    assert!(!writing.exists(), "the WAV in progress was left behind");
    assert!(!passage.exists(), "the passage was left behind");
    assert_eq!(listing(&out), Vec::<String>::new());
}
