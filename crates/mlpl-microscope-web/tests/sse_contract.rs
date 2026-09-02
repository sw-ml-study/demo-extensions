use mlpl_microscope_model::Budgets;
use mlpl_microscope_web::{LiveAssembler, LiveEvent, LivePhase, SseParser};

const STREAM: &str = "event: ready\ndata: {}\n\nevent: frame\ndata: {\"name\":\"group/value\",\"step\":2,\"shape\":[2],\"values\":[1,2]}\n\nevent: frame\ndata: {\"name\":\"group/scalar\",\"step\":2,\"shape\":[],\"values\":[3]}\n\nevent: frame\ndata: {\"name\":\"group/value\",\"step\":4,\"shape\":[2],\"values\":[4,5]}\n\nevent: done\ndata: {}\n\n";

fn budgets() -> Budgets {
    Budgets {
        max_frames: 4,
        max_observations_per_frame: 4,
        max_values_per_observation: 4,
        max_total_values: 12,
    }
}

#[test]
fn every_byte_split_assembles_identically() {
    let expected = parse_chunks([STREAM.as_bytes()]);
    for split in 0..=STREAM.len() {
        assert_eq!(
            parse_chunks([&STREAM.as_bytes()[..split], &STREAM.as_bytes()[split..]]),
            expected,
            "split {split}"
        );
    }
}

#[test]
fn ready_frames_done_preserve_call_order_and_nonconsecutive_steps() {
    let events = parse_chunks([STREAM.as_bytes()]);
    let mut assembler = LiveAssembler::new("generic", budgets());
    for event in &events {
        assembler
            .apply(LiveAssembler::decode(event).unwrap())
            .unwrap();
    }
    assert_eq!(assembler.phase, LivePhase::Done);
    let recording = assembler.recording().unwrap();
    assert_eq!(
        recording
            .frames
            .iter()
            .map(|frame| frame.step)
            .collect::<Vec<_>>(),
        [2, 4]
    );
    assert_eq!(
        recording.frames[0]
            .observations
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        ["group/value", "group/scalar"]
    );
}

#[test]
fn error_flow_and_budget_failure_are_visible_without_partial_retention() {
    let mut assembler = LiveAssembler::new("generic", budgets());
    assembler.apply(LiveEvent::Ready).unwrap();
    assembler
        .apply(LiveEvent::Error("producer failed".into()))
        .unwrap();
    assert_eq!(assembler.phase, LivePhase::Failed("producer failed".into()));
    assert!(assembler.recording().is_err());

    let over = mlpl_microscope_model::Observation {
        name: "too-large".into(),
        shape: vec![5],
        values: vec![0.0; 5],
    };
    assert!(
        assembler
            .apply(LiveEvent::Frame {
                step: 0,
                observation: over
            })
            .is_err()
    );
}

fn parse_chunks<'a>(
    chunks: impl IntoIterator<Item = &'a [u8]>,
) -> Vec<mlpl_microscope_web::SseEvent> {
    let mut parser = SseParser::default();
    let mut events = Vec::new();
    for chunk in chunks {
        events.extend(parser.push(chunk).unwrap());
    }
    parser.finish().unwrap();
    events
}
