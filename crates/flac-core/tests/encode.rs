//! Encoder conformance: re-encode real audio and prove it round-trips
//! losslessly through our own decoder, sample-for-sample.

use kryx_flac::{decode, encode, EncodeOptions};

fn roundtrip(flac: &[u8], name: &str) {
    let original = decode(flac).expect("decode original");
    let opts = EncodeOptions::new(original.info.sample_rate, original.info.bits_per_sample);
    let encoded = encode(&original.channels, &opts).expect("encode");
    let back = decode(&encoded).expect("decode re-encoded");

    assert_eq!(
        back.channels, original.channels,
        "{name}: re-encoded audio is not lossless"
    );
    assert_eq!(
        back.info.sample_rate, original.info.sample_rate,
        "{name}: sr"
    );
    assert_eq!(back.info.channels, original.info.channels, "{name}: ch");
    assert_eq!(
        back.info.bits_per_sample, original.info.bits_per_sample,
        "{name}: bps"
    );
}

#[test]
fn roundtrip_constant() {
    roundtrip(include_bytes!("fixtures/const.flac"), "const");
}

#[test]
fn roundtrip_noise_mono() {
    roundtrip(include_bytes!("fixtures/noise.flac"), "noise");
}

#[test]
fn roundtrip_stereo() {
    roundtrip(include_bytes!("fixtures/stereo.flac"), "stereo");
}

#[test]
fn roundtrip_synthetic_ramp_and_tone() {
    // A multi-frame signal that exercises FIXED orders and partitioning.
    let n = 10_000usize;
    let ch0: Vec<i32> = (0..n).map(|i| ((i as i32 * 7) % 2000) - 1000).collect();
    let ch1: Vec<i32> = (0..n)
        .map(|i| ((i as f64 * 0.05).sin() * 8000.0) as i32)
        .collect();
    let channels = vec![ch0, ch1];

    let opts = EncodeOptions::new(44_100, 16);
    let encoded = encode(&channels, &opts).expect("encode");
    let back = decode(&encoded).expect("decode");
    assert_eq!(back.channels, channels, "synthetic: not lossless");
    assert_eq!(back.frame_count(), n);
}

#[test]
fn encodes_tags() {
    let channels = vec![vec![0i32; 200]];
    let mut opts = EncodeOptions::new(8000, 16);
    opts.tags = vec![("TITLE".into(), "Kryx".into())];
    let encoded = encode(&channels, &opts).expect("encode");

    let meta = kryx_flac::read_metadata(&encoded).expect("metadata");
    assert_eq!(meta.tag("TITLE"), Some("Kryx"));
}
