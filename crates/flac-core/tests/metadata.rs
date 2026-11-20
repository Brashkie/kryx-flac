//! Conformance for M3: metadata (tags, pictures, seek table), CRC verification,
//! and sample-range decoding. Fixtures are committed next to this test; the
//! tagged/seek files carry a 440 Hz tone, cover art and VORBIS_COMMENT tags.

use kryx_flac::{decode, decode_range, read_metadata, FlacError};

const TAGGED: &[u8] = include_bytes!("fixtures/tagged.flac");
const SEEK: &[u8] = include_bytes!("fixtures/seek.flac");
const CORRUPT: &[u8] = include_bytes!("fixtures/corrupt.flac");
const STEREO: &[u8] = include_bytes!("fixtures/stereo.flac");

#[test]
fn reads_vorbis_comment_tags() {
    let m = read_metadata(TAGGED).expect("metadata");
    assert_eq!(m.tag("TITLE"), Some("Kryx Test Tone"));
    assert_eq!(m.tag("ARTIST"), Some("Brashkie"));
    assert_eq!(m.tag("album"), Some("KryxJS")); // case-insensitive lookup
    assert!(!m.vendor.is_empty());
}

#[test]
fn reads_embedded_picture() {
    let m = read_metadata(TAGGED).expect("metadata");
    assert_eq!(m.pictures.len(), 1);
    let pic = &m.pictures[0];
    assert_eq!(pic.mime, "image/png");
    assert!(!pic.data.is_empty());
    // PNG magic signature.
    assert_eq!(&pic.data[..4], &[0x89, b'P', b'N', b'G']);
}

#[test]
fn tags_survive_full_decode() {
    let audio = decode(TAGGED).expect("decode");
    assert_eq!(audio.tag("TITLE"), Some("Kryx Test Tone"));
    assert_eq!(audio.pictures.len(), 1);
    assert_eq!(audio.info.total_samples, 8000);
    assert_eq!(audio.frame_count(), 8000);
}

#[test]
fn reads_seek_table() {
    let m = read_metadata(SEEK).expect("metadata");
    assert!(!m.seek_points.is_empty());
    // Points are sorted and reference real samples within the stream.
    assert!(m.seek_points.windows(2).all(|w| w[0].sample < w[1].sample));
    assert!(m.seek_points.iter().all(|p| p.sample < 8000));
}

#[test]
fn decode_range_matches_full_decode() {
    let full = decode(SEEK).expect("decode").interleaved();
    let ch = 2usize;

    // A window that spans a seek point (frames are 576 samples).
    for &(start, count) in &[(0u64, 100u64), (600, 500), (4000, 1000), (7900, 100)] {
        let got = decode_range(SEEK, start, count)
            .expect("range")
            .interleaved();
        let s = start as usize * ch;
        let e = ((start + count) as usize).min(8000) * ch;
        assert_eq!(
            got.as_slice(),
            &full[s..e],
            "window start={start} count={count}"
        );
    }
}

#[test]
fn detects_crc_corruption() {
    // A byte flipped inside an audio frame must be caught by CRC verification.
    match decode(CORRUPT) {
        Err(FlacError::CrcMismatch(_)) => {}
        other => panic!("expected CrcMismatch, got {other:?}"),
    }
}

#[test]
fn plain_stream_has_no_pictures_or_seek_table() {
    // (FFmpeg always writes a VORBIS_COMMENT with an ENCODER tag, so tags are
    // not necessarily empty — but there is no cover art and no seek table.)
    let m = read_metadata(STEREO).expect("metadata");
    assert!(m.pictures.is_empty());
    assert!(m.seek_points.is_empty());
    assert_eq!(m.tag("TITLE"), None);
    assert_eq!(m.stream_info.channels, 2);
}
