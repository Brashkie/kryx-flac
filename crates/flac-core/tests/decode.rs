//! Conformance: decode real FLAC files and compare, sample-for-sample, against
//! the PCM that FFmpeg produces from the same files. The fixtures are committed
//! next to this test.

use kryx_flac::decode;

/// Decode `flac`, interleave, narrow to 16-bit LE bytes, and compare to `pcm`.
fn check(flac: &[u8], pcm: &[u8], name: &str) {
    let audio = decode(flac).expect("decode");
    let interleaved = audio.interleaved();

    let mut got = Vec::with_capacity(interleaved.len() * 2);
    for s in interleaved {
        got.extend_from_slice(&(s as i16).to_le_bytes());
    }

    assert_eq!(got.len(), pcm.len(), "{name}: byte length");
    assert_eq!(got, pcm, "{name}: samples differ from FFmpeg reference");
}

#[test]
fn decodes_constant_subframe() {
    check(
        include_bytes!("fixtures/const.flac"),
        include_bytes!("fixtures/const.pcm"),
        "const",
    );
}

#[test]
fn decodes_high_entropy_mono() {
    check(
        include_bytes!("fixtures/noise.flac"),
        include_bytes!("fixtures/noise.pcm"),
        "noise",
    );
}

#[test]
fn decodes_stereo_with_decorrelation() {
    check(
        include_bytes!("fixtures/stereo.flac"),
        include_bytes!("fixtures/stereo.pcm"),
        "stereo",
    );
}

#[test]
fn reports_stream_info() {
    let audio = decode(include_bytes!("fixtures/stereo.flac")).unwrap();
    assert_eq!(audio.info.sample_rate, 8_000);
    assert_eq!(audio.info.channels, 2);
    assert_eq!(audio.info.bits_per_sample, 16);
    assert_eq!(audio.frame_count(), audio.info.total_samples as usize);
}
