//! State determinacy across readers and comparison — #42.

mod common;
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn event(kind: &str, value: u32) -> TrackEventKind<'static> {
    match kind {
        "tempo" => TrackEventKind::Meta(MetaMessage::Tempo(value.into())),
        "program" => TrackEventKind::Midi {
            channel: 0.into(),
            message: MidiMessage::ProgramChange {
                program: (value as u8).into(),
            },
        },
        "controller" => TrackEventKind::Midi {
            channel: 0.into(),
            message: MidiMessage::Controller {
                controller: 11.into(),
                value: (value as u8).into(),
            },
        },
        _ => unreachable!(),
    }
}

fn take(dir: &Path, name: &str, kind: &str, left: &[(u32, u32)], right: &[(u32, u32)]) -> PathBuf {
    let mut tracks = vec![vec![
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(2, 2, 24, 8)),
        },
        TrackEvent {
            delta: 3840.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        },
    ]];
    for statements in [left, right] {
        let mut events = Vec::new();
        let mut previous = 0;
        for &(tick, value) in statements {
            events.push(TrackEvent {
                delta: (tick - previous).into(),
                kind: event(kind, value),
            });
            previous = tick;
        }
        events.push(TrackEvent {
            delta: (3840 - previous).into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });
        tracks.push(events);
    }
    let path = dir.join(name);
    Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(480.into())),
        tracks,
    }
    .save(&path)
    .unwrap();
    path
}

fn output(args: &[&str]) -> Value {
    serde_json::from_str(&common::json_output(args)).unwrap()
}

fn append_track(path: &Path, statements: &[(u32, TrackEventKind<'static>)]) {
    let bytes = std::fs::read(path).unwrap();
    let mut smf = Smf::parse(&bytes).unwrap();
    let mut previous = 0;
    let mut events = Vec::new();
    for &(tick, kind) in statements {
        events.push(TrackEvent {
            delta: (tick - previous).into(),
            kind,
        });
        previous = tick;
    }
    events.push(TrackEvent {
        delta: (3840 - previous).into(),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    smf.tracks.push(events);
    smf.save(path).unwrap();
}

#[test]
fn same_track_final_values_and_cross_track_agreement_remain_determinate() {
    let dir = tempfile::tempdir().unwrap();
    for kind in ["tempo", "program", "controller"] {
        let (low, high) = if kind == "tempo" {
            (500000, 1000000)
        } else {
            (0, 80)
        };
        let a = take(
            dir.path(),
            "a.mid",
            kind,
            &[(0, low), (0, high)],
            &[(0, high)],
        );
        let b = take(dir.path(), "b.mid", kind, &[(0, high)], &[]);
        let reversed = take(
            dir.path(),
            "reversed.mid",
            kind,
            &[(0, high), (0, low)],
            &[],
        );
        let read = battuta::Take::read(&a).unwrap();
        let expected = battuta::Reading::Determinate { value: high as u8 };
        match kind {
            "tempo" => assert_eq!(
                read.info().unwrap().tempo,
                battuta::Reading::Determinate {
                    value: battuta::Tempo::from_micros_per_quarter(high)
                }
            ),
            "program" => assert_eq!(
                read.programs_in(None).unwrap().programs[0].program,
                expected
            ),
            _ => assert_eq!(
                read.controllers_in(None).unwrap().controllers[0].value,
                expected
            ),
        }
        let same = battuta::diff::diff(&read, &battuta::Take::read(&b).unwrap(), None).unwrap();
        assert!(same.is_empty() && !same.has_unranked(), "{same:?}");
        let changed =
            battuta::diff::diff(&read, &battuta::Take::read(&reversed).unwrap(), None).unwrap();
        assert!(
            !changed.is_empty() && !changed.has_unranked(),
            "{changed:?}"
        );
    }
}

#[test]
fn unstated_is_distinct_from_an_explicit_zero_or_default_tempo() {
    let dir = tempfile::tempdir().unwrap();
    for kind in ["tempo", "program", "controller"] {
        let value = if kind == "tempo" { 500000 } else { 0 };
        let empty = take(dir.path(), "empty.mid", kind, &[], &[]);
        let stated = take(dir.path(), "stated.mid", kind, &[(0, value)], &[]);
        assert_eq!(
            output(&["info", empty.to_str().unwrap(), "--json"])["tempo"],
            json!({"kind":"unstated"})
        );
        let diff = output(&[
            "diff",
            empty.to_str().unwrap(),
            stated.to_str().unwrap(),
            "--json",
        ]);
        let rows = diff[format!("{kind}s")].as_array().unwrap();
        assert_eq!(rows.len(), 1);
        if kind == "program" {
            assert!(rows[0]["before"].is_null());
            assert_eq!(rows[0]["after"], 0);
        } else {
            assert!(rows[0]["before"]["at_start"].is_null());
            assert!(!rows[0]["after"]["at_start"].is_null());
        }
        assert_eq!(diff[format!("unranked_{kind}s")], json!([]));
    }
}

#[test]
fn a_controller_conflict_does_not_hide_other_channels_or_controller_numbers() {
    let dir = tempfile::tempdir().unwrap();
    let a = take(dir.path(), "a.mid", "controller", &[(0, 20)], &[(0, 80)]);
    let b = take(dir.path(), "b.mid", "controller", &[(0, 20)], &[]);
    for (file, value) in [(&a, 30u8), (&b, 40u8)] {
        append_track(
            file,
            &[
                (
                    0,
                    TrackEventKind::Midi {
                        channel: 1.into(),
                        message: MidiMessage::Controller {
                            controller: 11.into(),
                            value: value.into(),
                        },
                    },
                ),
                (
                    0,
                    TrackEventKind::Midi {
                        channel: 0.into(),
                        message: MidiMessage::Controller {
                            controller: 7.into(),
                            value: value.into(),
                        },
                    },
                ),
            ],
        );
    }
    let diff = battuta::diff::diff(
        &battuta::Take::read(&a).unwrap(),
        &battuta::Take::read(&b).unwrap(),
        None,
    )
    .unwrap();
    assert_eq!(
        diff.controllers
            .iter()
            .map(|row| (row.channel, row.controller))
            .collect::<Vec<_>>(),
        vec![(0, 7), (1, 11)]
    );
    assert_eq!(diff.unranked_controllers.len(), 1);
    let excluded = &diff.unranked_controllers[0];
    assert_eq!((excluded.channel, excluded.controller), (0, 11));
    assert_eq!(excluded.interval.before.len(), 2);
    assert!(excluded.interval.after.is_empty());
    assert!(!diff.is_empty() && diff.has_unranked());
}

#[test]
fn state_strike_sites_are_side_aware_and_do_not_turn_into_value_intervals() {
    let dir = tempfile::tempdir().unwrap();
    for kind in ["program", "controller"] {
        let a = take(dir.path(), "a.mid", kind, &[(0, 40), (480, 40)], &[]);
        let b = take(dir.path(), "b.mid", kind, &[(0, 40)], &[]);
        for file in [&a, &b] {
            append_track(file, &[common::strike(480, 60), common::release(720, 60)]);
        }
        for (before, after, in_before) in [(&a, &b, true), (&b, &a, false)] {
            let diff = battuta::diff::diff(
                &battuta::Take::read(before).unwrap(),
                &battuta::Take::read(after).unwrap(),
                None,
            )
            .unwrap();
            assert!(diff.is_empty() && diff.has_unranked());
            assert!(diff.unranked_programs.is_empty() && diff.unranked_controllers.is_empty());
            assert_eq!(diff.unranked_state_sites.len(), 1);
            let site = &diff.unranked_state_sites[0];
            assert_eq!((site.in_before, site.in_after), (in_before, !in_before));
            assert_eq!(
                (site.site.tick, site.site.track, site.site.against_track),
                (480, 1, 3)
            );
            assert_eq!(site.site.against, battuta::Against::Notes);
            let human =
                common::human_output(&["diff", before.to_str().unwrap(), after.to_str().unwrap()]);
            assert!(human.contains("no determinate differences"), "{human}");
        }
    }
}

#[test]
fn state_readers_inherit_conflicts_and_resume_after_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    for kind in ["tempo", "program", "controller"] {
        let (low, high) = if kind == "tempo" {
            (500000, 1000000)
        } else {
            (20, 80)
        };
        let file = take(
            dir.path(),
            &format!("{kind}.mid"),
            kind,
            &[(480, low), (1920, low)],
            &[(480, high)],
        );
        let name = file.to_str().unwrap();
        let middle = output(&["inspect", name, "--bars", "2:2", "--json"]);
        let later = output(&["inspect", name, "--bars", "3:3", "--json"]);
        if kind == "tempo" {
            assert_eq!(
                output(&["info", name, "--json"])["tempo"]["kind"],
                "indeterminate"
            );
            assert_eq!(middle["unranked_tempos"][0]["from"], 960);
            assert_eq!(middle["unranked_tempos"][0]["until"], 1920);
            assert_eq!(later["unranked_tempos"], json!([]));
        } else {
            let list = format!("{kind}s");
            let field = if kind == "program" {
                "program"
            } else {
                "value"
            };
            assert_eq!(middle[&list][0][field]["kind"], "indeterminate");
            assert_eq!(middle[&list][0]["unranked"][0]["from"], 960);
            assert_eq!(
                middle[&list][0]["unranked"][0]["candidates"][0]["tick"],
                480
            );
            assert_eq!(
                later[&list][0][field],
                json!({"kind":"determinate", "value":low})
            );
            assert_eq!(later[&list][0]["unranked"], json!([]));
            if kind == "controller" {
                assert_eq!(middle[&list][0]["peak"]["kind"], "incomplete");
            }
        }
        let human = common::human_output(&["inspect", name, "--bars", "2:2"]);
        assert!(
            human.contains("indeterminate")
                && human.contains("track 1")
                && human.contains("track 2"),
            "{human}"
        );
    }
}

#[test]
fn state_self_comparison_and_track_swaps_disclose_without_choosing() {
    let dir = tempfile::tempdir().unwrap();
    for kind in ["tempo", "program", "controller"] {
        let (low, high) = if kind == "tempo" {
            (500000, 1000000)
        } else {
            (20, 80)
        };
        let a = take(dir.path(), "a.mid", kind, &[(0, low)], &[(0, high)]);
        let b = take(dir.path(), "b.mid", kind, &[(0, high)], &[(0, low)]);
        let whole = output(&["inspect", a.to_str().unwrap(), "--json"]);
        let intervals = if kind == "tempo" {
            &whole["unranked_tempos"]
        } else {
            &whole[format!("{kind}s")][0]["unranked"]
        };
        assert!(intervals[0]["until"].is_null(), "{whole}");
        for other in [&a, &b] {
            let diff = output(&[
                "diff",
                a.to_str().unwrap(),
                other.to_str().unwrap(),
                "--json",
            ]);
            assert_eq!(diff[format!("{kind}s")], json!([]));
            let intervals = &diff[format!("unranked_{kind}s")];
            assert_eq!(intervals[0]["from"], 0);
            assert_eq!(intervals[0]["before"].as_array().unwrap().len(), 2);
            assert_eq!(intervals[0]["after"].as_array().unwrap().len(), 2);
            let human =
                common::human_output(&["diff", a.to_str().unwrap(), other.to_str().unwrap()]);
            assert!(human.contains("no determinate differences"), "{human}");
        }
    }
}

#[test]
fn state_differences_stop_at_conflicts_and_resume_without_false_extremes() {
    let dir = tempfile::tempdir().unwrap();
    for kind in ["tempo", "program", "controller"] {
        let (low, high, base) = if kind == "tempo" {
            (500000, 1000000, 600000)
        } else {
            (20, 80, 30)
        };
        let a = take(
            dir.path(),
            "a.mid",
            kind,
            &[(0, low), (480, low), (1920, low)],
            &[(480, high)],
        );
        let b = take(dir.path(), "b.mid", kind, &[(0, base)], &[]);
        if kind == "controller" {
            let inspect = output(&["inspect", a.to_str().unwrap(), "--json"]);
            assert_eq!(
                inspect["controllers"][0]["value"],
                json!({"kind":"determinate", "value":low})
            );
            assert_eq!(
                inspect["controllers"][0]["peak"],
                json!({"kind":"incomplete"})
            );
        }
        for (before, after) in [(&a, &b), (&b, &a)] {
            let diff = output(&[
                "diff",
                before.to_str().unwrap(),
                after.to_str().unwrap(),
                "--json",
            ]);
            let rows = diff[format!("{kind}s")].as_array().unwrap();
            let start = if kind == "program" { "at" } else { "from" };
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0][start], 0);
            assert_eq!(rows[0]["until"], 480);
            assert_eq!(rows[1][start], 1920);
            assert!(rows[1]["until"].is_null());
            assert_eq!(diff[format!("unranked_{kind}s")][0]["until"], 1920);
            let uncertain_side = if before == &a { "before" } else { "after" };
            let determinate_side = if before == &a { "after" } else { "before" };
            let interval = &diff[format!("unranked_{kind}s")][0];
            assert_eq!(interval[uncertain_side].as_array().unwrap().len(), 2);
            assert_eq!(interval[determinate_side], json!([]));
            for row in rows {
                match kind {
                    "controller" => assert_eq!(row[uncertain_side]["peak"], low),
                    "tempo" => {
                        assert_eq!(row[uncertain_side]["slowest"]["micros_per_quarter"], low)
                    }
                    _ => assert_eq!(row[uncertain_side], low),
                }
            }
        }
    }
}
