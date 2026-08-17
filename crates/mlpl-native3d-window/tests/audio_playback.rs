use mlpl_native3d_window::audio::PlaybackBuffer;

#[test]
fn playback_buffer_resamples_stereo_incrementally_and_stays_bounded() {
    let mut buffer = PlaybackBuffer::new(6);
    buffer.push_stereo(&[0.0, 1.0], &[1.0, 0.0], 2, 4);
    assert_eq!(buffer.len(), 6, "bounded interleaved sample capacity");
    let mut output = [0.0; 8];
    buffer.fill(&mut output);
    assert_eq!(&output[..6], &[0.5, 0.5, 1.0, 0.0, 1.0, 0.0]);
    assert_eq!(&output[6..], &[0.0, 0.0], "underrun is silence");
    assert_eq!(buffer.len(), 0);
}

#[test]
fn playback_flush_discards_pre_seek_audio() {
    let mut buffer = PlaybackBuffer::new(8);
    buffer.push_stereo(&[0.25, 0.5], &[0.75, 1.0], 44_100, 44_100);
    assert_eq!(buffer.len(), 4);
    buffer.clear();
    assert_eq!(buffer.len(), 0);
}
