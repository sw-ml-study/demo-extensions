use mlpl_native3d_window::audio::{DecodeLimits, PcmStream, discover_audio_paths};

#[test]
fn mp3_is_decoded_as_bounded_incremental_stereo_pcm() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../demo-file-processing/fixtures/mp3/tone-vbr-id3.mp3");
    let mut stream = PcmStream::open(
        &path,
        DecodeLimits {
            max_frames_per_chunk: 512,
            max_channels: 2,
        },
    )
    .expect("decodable MP3 fixture");
    let first = stream
        .next_chunk()
        .expect("decode succeeds")
        .expect("PCM chunk");
    assert_eq!(first.left.len(), first.right.len());
    assert!(!first.left.is_empty());
    assert!(first.left.len() <= 512);
    assert!(first.sample_rate_hz > 0);
    assert_eq!(first.start_frame, 0);
    assert!(first.left.iter().any(|sample| sample.abs() > 0.000_001));
}

#[test]
fn complete_fixture_decode_never_exceeds_the_chunk_bound() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../demo-file-processing/fixtures/mp3/tone-vbr-id3.mp3");
    let mut stream = PcmStream::open(
        &path,
        DecodeLimits {
            max_frames_per_chunk: 1024,
            max_channels: 2,
        },
    )
    .unwrap();
    let started = std::time::Instant::now();
    let mut chunks = 0;
    let mut frames = 0;
    while let Some(chunk) = stream.next_chunk().unwrap() {
        assert!(chunk.left.len() <= 1024);
        chunks += 1;
        frames += chunk.left.len();
    }
    eprintln!(
        "decoded {frames} stereo frames in {chunks} bounded chunks in {:?}",
        started.elapsed()
    );
    assert!(chunks > 1);
    assert!(frames > 1024);
}

#[test]
fn decode_limits_reject_unbounded_or_missing_inputs() {
    let path = std::path::Path::new("does-not-exist.mp3");
    assert!(
        PcmStream::open(
            path,
            DecodeLimits {
                max_frames_per_chunk: 0,
                max_channels: 2
            }
        )
        .is_err()
    );
    assert!(
        PcmStream::open(
            path,
            DecodeLimits {
                max_frames_per_chunk: 512,
                max_channels: 0
            }
        )
        .is_err()
    );
}

#[test]
fn picker_catalog_combines_mp3_and_ogg_with_a_hard_bound() {
    let root = std::env::temp_dir().join(format!("audio-catalog-{}", std::process::id()));
    std::fs::create_dir_all(root.join("nested")).unwrap();
    std::fs::write(root.join("b.ogg"), []).unwrap();
    std::fs::write(root.join("nested/a.mp3"), []).unwrap();
    std::fs::write(root.join("ignored.wav"), []).unwrap();
    assert_eq!(
        discover_audio_paths(&root, 8).unwrap(),
        ["b.ogg", "nested/a.mp3"]
    );
    assert_eq!(discover_audio_paths(&root, 1).unwrap().len(), 1);
    std::fs::remove_dir_all(root).ok();
}
