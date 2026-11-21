//! A pure-Rust FLAC **encoder** (M4).
//!
//! Lossless, zero third-party. Per subframe it picks the cheapest of CONSTANT,
//! VERBATIM and the five FIXED predictors (orders 0–4), codes the residual with
//! a partitioned Rice search, and writes valid frame headers with CRC-8/CRC-16.
//! Channels are stored independently. LPC and stereo decorrelation are later
//! work (they improve ratio, not correctness).

use crate::bitwriter::BitWriter;
use crate::crc::{crc16, crc8};
use crate::error::{FlacError, Result};

/// Encoder settings.
#[derive(Debug, Clone)]
pub struct EncodeOptions {
    /// Samples per block (per channel). Default 4096.
    pub block_size: u32,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Bits per sample (4..=32).
    pub bits_per_sample: u8,
    /// VORBIS_COMMENT tags to embed as `(KEY, value)` pairs.
    pub tags: Vec<(String, String)>,
}

impl EncodeOptions {
    /// Options for the given format with a 4096-sample block and no tags.
    pub fn new(sample_rate: u32, bits_per_sample: u8) -> Self {
        Self {
            block_size: 4096,
            sample_rate,
            bits_per_sample,
            tags: Vec::new(),
        }
    }
}

const MAX_PARTITION_ORDER: u32 = 6;
const FIXED_MAX_ORDER: usize = 4;
const MAX_LPC_ORDER: usize = 12;
const LPC_PRECISION: u32 = 15;

/// Encode planar channels (`channels[c][i]`) to a complete FLAC byte stream.
/// Every channel vector must be the same length.
pub fn encode(channels: &[Vec<i32>], opts: &EncodeOptions) -> Result<Vec<u8>> {
    let nch = channels.len();
    if nch == 0 || nch > 8 {
        return Err(FlacError::Reserved("channel count"));
    }
    if !(4..=32).contains(&opts.bits_per_sample) {
        return Err(FlacError::Reserved("bits per sample"));
    }
    let total = channels[0].len();
    if channels.iter().any(|c| c.len() != total) {
        return Err(FlacError::Reserved("ragged channels"));
    }
    let bs = opts.block_size.max(16) as usize;
    let bps = opts.bits_per_sample as u32;

    // Determine min/max block size actually used (last frame may be short).
    let last = if total == 0 {
        0
    } else {
        ((total - 1) % bs) + 1
    };
    let min_block = if total <= bs { total } else { last.min(bs) } as u32;
    let max_block = bs as u32;

    let mut w = BitWriter::new();
    w.write(0x664C_6143, 32); // "fLaC"
    write_stream_info(&mut w, opts, nch, total as u64, min_block, max_block);
    if !opts.tags.is_empty() {
        write_vorbis_comment(&mut w, opts);
    }

    let mut frame_number: u64 = 0;
    let mut pos = 0;
    while pos < total {
        let n = bs.min(total - pos);
        write_frame(&mut w, channels, pos, n, bps, nch, frame_number, opts);
        pos += n;
        frame_number += 1;
    }

    Ok(w.finish())
}

/// Convenience: encode interleaved samples (`ch0[0], ch1[0], …`).
pub fn encode_interleaved(
    samples: &[i32],
    channels: usize,
    opts: &EncodeOptions,
) -> Result<Vec<u8>> {
    if channels == 0 || samples.len() % channels != 0 {
        return Err(FlacError::Reserved("interleaved length"));
    }
    let n = samples.len() / channels;
    let mut planar: Vec<Vec<i32>> = (0..channels).map(|_| Vec::with_capacity(n)).collect();
    for (i, &s) in samples.iter().enumerate() {
        planar[i % channels].push(s);
    }
    encode(&planar, opts)
}

fn write_stream_info(
    w: &mut BitWriter,
    opts: &EncodeOptions,
    nch: usize,
    total: u64,
    min_block: u32,
    max_block: u32,
) {
    // Metadata block header: last? (set only if no VORBIS_COMMENT follows).
    let last = opts.tags.is_empty();
    w.write(last as u64, 1);
    w.write(0, 7); // type 0 = STREAMINFO
    w.write(34, 24); // length

    w.write(min_block as u64, 16);
    w.write(max_block as u64, 16);
    w.write(0, 24); // min frame size (unknown)
    w.write(0, 24); // max frame size (unknown)
    w.write(opts.sample_rate as u64, 20);
    w.write((nch as u64) - 1, 3);
    w.write((opts.bits_per_sample as u64) - 1, 5);
    w.write_wide(total, 36);
    // MD5 of the unencoded audio: 0 = "not computed" (allowed by the spec).
    for _ in 0..16 {
        w.write(0, 8);
    }
}

fn write_vorbis_comment(w: &mut BitWriter, opts: &EncodeOptions) {
    let vendor = b"kryx-flac";
    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    body.extend_from_slice(vendor);
    body.extend_from_slice(&(opts.tags.len() as u32).to_le_bytes());
    for (k, v) in &opts.tags {
        let line = format!("{k}={v}");
        body.extend_from_slice(&(line.len() as u32).to_le_bytes());
        body.extend_from_slice(line.as_bytes());
    }
    w.write(1, 1); // last block
    w.write(4, 7); // type 4 = VORBIS_COMMENT
    w.write(body.len() as u64, 24);
    for b in body {
        w.write(b as u64, 8);
    }
}

#[allow(clippy::too_many_arguments)]
fn write_frame(
    w: &mut BitWriter,
    channels: &[Vec<i32>],
    pos: usize,
    n: usize,
    bps: u32,
    nch: usize,
    frame_number: u64,
    _opts: &EncodeOptions,
) {
    // Decide channel assignment + per-subframe encoding before the header,
    // because the channel-assignment code lives in the header.
    let (ch_code, subs) = plan_frame(channels, pos, n, bps, nch);

    let frame_start = w.byte_len();

    // --- Frame header ---
    w.write(0x3FFE, 14); // sync
    w.write(0, 1); // reserved
    w.write(0, 1); // blocking strategy: fixed block size
                   // Block size code: 6 => 8-bit (n-1), 7 => 16-bit (n-1).
    let (bs_code, bs_bits) = if n - 1 < 256 {
        (6u64, 8u32)
    } else {
        (7u64, 16u32)
    };
    w.write(bs_code, 4);
    w.write(0, 4); // sample rate: 0 => from STREAMINFO
    w.write(ch_code, 4); // channel assignment
    w.write(0, 3); // sample size: 0 => from STREAMINFO
    w.write(0, 1); // reserved
    write_utf8(w, frame_number);
    w.write((n - 1) as u64, bs_bits);
    // (no sample-rate trailer because sr code is 0)
    let header_crc = crc8(&w.bytes_slice()[frame_start..]);
    w.write(header_crc as u64, 8);

    // --- Subframes ---
    for cp in &subs {
        emit_subframe(w, cp);
    }

    // --- Frame footer: CRC-16 over the whole frame ---
    w.align_byte();
    let frame_crc = crc16(&w.bytes_slice()[frame_start..]);
    w.write(frame_crc as u64, 16);
}

/// A fully-formed LPC candidate for a subframe.
struct LpcCandidate {
    order: usize,
    shift: i32,
    qcoeff: Vec<i32>,
    residual: Vec<i64>,
    bits: u64,
}

/// The chosen encoding of one subframe.
enum Sub {
    Constant(i32),
    Verbatim,
    Fixed { order: usize, residual: Vec<i64> },
    Lpc(LpcCandidate),
}

/// A subframe's samples, its bit depth, the chosen encoding and its bit cost.
struct ChannelPlan {
    x: Vec<i32>,
    bps: u32,
    sub: Sub,
}

/// Plan a whole frame: pick the stereo channel assignment (for 2 channels) and
/// each subframe's encoding. Returns `(channel_assignment_code, subframes)`.
fn plan_frame(
    channels: &[Vec<i32>],
    pos: usize,
    n: usize,
    bps: u32,
    nch: usize,
) -> (u64, Vec<ChannelPlan>) {
    if nch == 2 {
        let l: Vec<i32> = channels[0][pos..pos + n].to_vec();
        let r: Vec<i32> = channels[1][pos..pos + n].to_vec();
        let side: Vec<i32> = (0..n).map(|i| l[i] - r[i]).collect();
        let mid: Vec<i32> = (0..n)
            .map(|i| ((l[i] as i64 + r[i] as i64) >> 1) as i32)
            .collect();

        let pl = plan_subframe(&l, bps);
        let pr = plan_subframe(&r, bps);
        let pm = plan_subframe(&mid, bps);
        let ps = plan_subframe(&side, bps + 1);

        let indep = pl.bits + pr.bits;
        let left_side = pl.bits + ps.bits;
        let right_side = ps.bits + pr.bits;
        let mid_side = pm.bits + ps.bits;
        let best = indep.min(left_side).min(right_side).min(mid_side);

        let cp = |x: Vec<i32>, bps: u32, p: Plan| ChannelPlan { x, bps, sub: p.sub };
        if best == mid_side {
            (10, vec![cp(mid, bps, pm), cp(side, bps + 1, ps)])
        } else if best == left_side {
            (8, vec![cp(l, bps, pl), cp(side, bps + 1, ps)])
        } else if best == right_side {
            (9, vec![cp(side, bps + 1, ps), cp(r, bps, pr)])
        } else {
            (1, vec![cp(l, bps, pl), cp(r, bps, pr)])
        }
    } else {
        let mut subs = Vec::with_capacity(nch);
        for ch in channels.iter().take(nch) {
            let x = ch[pos..pos + n].to_vec();
            let p = plan_subframe(&x, bps);
            subs.push(ChannelPlan { x, bps, sub: p.sub });
        }
        ((nch - 1) as u64, subs)
    }
}

/// A planned subframe encoding plus its bit cost (header included).
struct Plan {
    bits: u64,
    sub: Sub,
}

/// Choose the cheapest of CONSTANT, FIXED, LPC, VERBATIM for one subframe.
fn plan_subframe(x: &[i32], bps: u32) -> Plan {
    let n = x.len();

    if n > 0 && x.iter().all(|&s| s == x[0]) {
        return Plan {
            bits: 8 + bps as u64,
            sub: Sub::Constant(x[0]),
        };
    }

    let (fixed_order, fixed_res) = choose_fixed(x);
    let fixed_bits = estimate_fixed_bits(x, fixed_order, &fixed_res, bps);
    let lpc = best_lpc(x, bps);
    let lpc_bits = lpc.as_ref().map_or(u64::MAX, |l| l.bits);
    let verbatim_bits = 8 + (n as u64) * bps as u64;

    if lpc_bits <= fixed_bits && lpc_bits < verbatim_bits {
        Plan {
            bits: lpc_bits,
            sub: Sub::Lpc(lpc.unwrap()),
        }
    } else if fixed_bits < verbatim_bits {
        Plan {
            bits: fixed_bits,
            sub: Sub::Fixed {
                order: fixed_order,
                residual: fixed_res,
            },
        }
    } else {
        Plan {
            bits: verbatim_bits,
            sub: Sub::Verbatim,
        }
    }
}

/// Write a planned subframe.
fn emit_subframe(w: &mut BitWriter, cp: &ChannelPlan) {
    let bps = cp.bps;
    let x = &cp.x;
    let n = x.len();
    match &cp.sub {
        Sub::Constant(v) => {
            w.write(0, 1);
            w.write(0, 6); // CONSTANT
            w.write(0, 1);
            w.write_signed(*v as i64, bps);
        }
        Sub::Verbatim => {
            w.write(0, 1);
            w.write(0b000001, 6); // VERBATIM
            w.write(0, 1);
            for &s in x {
                w.write_signed(s as i64, bps);
            }
        }
        Sub::Fixed { order, residual } => {
            w.write(0, 1);
            w.write(0b001000 | *order as u64, 6); // FIXED
            w.write(0, 1);
            for &s in x.iter().take(*order) {
                w.write_signed(s as i64, bps);
            }
            write_residual(w, residual, *order, n);
        }
        Sub::Lpc(l) => {
            w.write(0, 1);
            w.write(0b100000 | (l.order as u64 - 1), 6); // LPC
            w.write(0, 1);
            for &s in x.iter().take(l.order) {
                w.write_signed(s as i64, bps);
            }
            w.write((LPC_PRECISION - 1) as u64, 4);
            w.write_signed(l.shift as i64, 5);
            for &c in &l.qcoeff {
                w.write_signed(c as i64, LPC_PRECISION);
            }
            write_residual(w, &l.residual, l.order, n);
        }
    }
}

/// Build the best LPC candidate for `x`, or `None` when LPC doesn't apply.
fn best_lpc(x: &[i32], bps: u32) -> Option<LpcCandidate> {
    let n = x.len();
    let max_order = MAX_LPC_ORDER.min(n / 2);
    if max_order < 1 {
        return None;
    }

    // Windowed autocorrelation (Welch window).
    let windowed: Vec<f64> = welch_window(n)
        .iter()
        .zip(x)
        .map(|(wnd, &s)| wnd * s as f64)
        .collect();
    let autoc = autocorrelation(&windowed, max_order);
    if autoc[0] == 0.0 {
        return None;
    }
    let per_order = levinson(&autoc, max_order);

    let mut best: Option<LpcCandidate> = None;
    for coeffs in &per_order {
        let order = coeffs.len();
        let (qcoeff, shift) = quantize(coeffs, LPC_PRECISION);
        let residual = lpc_residual(x, &qcoeff, shift, order);
        let res_bits = plan_rice(&residual, order, n).bits;
        let bits =
            8 + order as u64 * bps as u64 + 4 + 5 + order as u64 * LPC_PRECISION as u64 + res_bits;
        if best.as_ref().map_or(true, |b| bits < b.bits) {
            best = Some(LpcCandidate {
                order,
                shift,
                qcoeff,
                residual,
                bits,
            });
        }
    }
    best
}

/// Welch window of length `n`.
fn welch_window(n: usize) -> Vec<f64> {
    if n <= 1 {
        return vec![1.0; n];
    }
    let half = (n - 1) as f64 / 2.0;
    (0..n)
        .map(|i| {
            let t = (i as f64 - half) / half;
            1.0 - t * t
        })
        .collect()
}

/// Autocorrelation of `s` for lags `0..=max_lag`.
fn autocorrelation(s: &[f64], max_lag: usize) -> Vec<f64> {
    let n = s.len();
    (0..=max_lag)
        .map(|lag| (lag..n).map(|i| s[i] * s[i - lag]).sum())
        .collect()
}

/// Levinson-Durbin: predictor coefficients for every order `1..=max_order`.
/// `out[o][j]` multiplies `x[i-1-j]` (lag `j+1`) for predictor order `o+1`.
fn levinson(autoc: &[f64], max_order: usize) -> Vec<Vec<f64>> {
    let mut err = autoc[0];
    let mut lpc = vec![0f64; max_order];
    let mut per_order = Vec::with_capacity(max_order);

    for i in 0..max_order {
        let mut acc = autoc[i + 1];
        for j in 0..i {
            acc -= lpc[j] * autoc[i - j];
        }
        if err == 0.0 {
            break;
        }
        let k = acc / err;
        lpc[i] = k;
        for j in 0..i.div_ceil(2) {
            let tmp = lpc[j];
            lpc[j] = tmp - k * lpc[i - 1 - j];
            if j != i - 1 - j {
                lpc[i - 1 - j] -= k * tmp;
            }
        }
        err *= 1.0 - k * k;
        per_order.push(lpc[..=i].to_vec());
        if err <= 0.0 {
            break;
        }
    }
    per_order
}

/// Quantize float LPC coefficients to `precision`-bit integers with a shift,
/// using error feedback. Always succeeds (clamps); losslessness is guaranteed
/// by the residual regardless.
fn quantize(lpc: &[f64], precision: u32) -> (Vec<i32>, i32) {
    let cmax = lpc.iter().fold(0f64, |m, &c| m.max(c.abs()));
    let max_shift = 15i32;
    let mut shift = if cmax > 0.0 {
        ((precision as f64 - 1.0) - cmax.log2()).floor() as i32
    } else {
        0
    };
    shift = shift.clamp(0, max_shift);

    let qmax = (1i64 << (precision - 1)) - 1;
    let qmin = -(1i64 << (precision - 1));
    let scale = (1i64 << shift) as f64;

    let mut q = Vec::with_capacity(lpc.len());
    let mut error = 0f64;
    for &c in lpc {
        let v = c * scale + error;
        let qi = (v.round() as i64).clamp(qmin, qmax);
        error = v - qi as f64;
        q.push(qi as i32);
    }
    (q, shift)
}

/// Integer LPC residual: `x[i] - ((Σ qcoeff[j]·x[i-1-j]) >> shift)`.
fn lpc_residual(x: &[i32], qcoeff: &[i32], shift: i32, order: usize) -> Vec<i64> {
    let n = x.len();
    let mut r = Vec::with_capacity(n - order);
    for i in order..n {
        let mut pred: i64 = 0;
        for (j, &c) in qcoeff.iter().enumerate() {
            pred += c as i64 * x[i - 1 - j] as i64;
        }
        let shifted = if shift >= 0 {
            pred >> shift
        } else {
            pred << (-shift)
        };
        r.push(x[i] as i64 - shifted);
    }
    r
}

/// FIXED predictor residuals for a given order (len = x.len() - order).
fn fixed_residual(x: &[i32], order: usize) -> Vec<i64> {
    let n = x.len();
    let mut r = Vec::with_capacity(n - order);
    for i in order..n {
        let v = x[i] as i64;
        let pred = match order {
            0 => 0,
            1 => x[i - 1] as i64,
            2 => 2 * x[i - 1] as i64 - x[i - 2] as i64,
            3 => 3 * x[i - 1] as i64 - 3 * x[i - 2] as i64 + x[i - 3] as i64,
            _ => 4 * x[i - 1] as i64 - 6 * x[i - 2] as i64 + 4 * x[i - 3] as i64 - x[i - 4] as i64,
        };
        r.push(v - pred);
    }
    r
}

/// Pick the FIXED order (0..=4) with the smallest sum of |residual|.
fn choose_fixed(x: &[i32]) -> (usize, Vec<i64>) {
    let max_order = FIXED_MAX_ORDER.min(x.len().saturating_sub(1));
    let mut best = (0usize, fixed_residual(x, 0));
    let mut best_sum = abs_sum(&best.1);
    for order in 1..=max_order {
        let r = fixed_residual(x, order);
        let s = abs_sum(&r);
        if s < best_sum {
            best_sum = s;
            best = (order, r);
        }
    }
    best
}

fn abs_sum(r: &[i64]) -> u128 {
    r.iter().map(|&v| v.unsigned_abs() as u128).sum()
}

fn estimate_fixed_bits(_x: &[i32], order: usize, residual: &[i64], bps: u32) -> u64 {
    let plan = plan_rice(residual, order, order + residual.len());
    8 + order as u64 * bps as u64 + plan.bits
}

/// Zig-zag map a signed residual to unsigned.
fn zigzag(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

/// A chosen partitioned-Rice layout: partition order, one parameter per
/// partition, and the estimated coded bit count including the field headers.
struct RicePlan {
    po: u32,
    params: Vec<u32>,
    bits: u64,
}

/// Best Rice parameter for a partition from its zig-zag `sum` and sample
/// `count`, using the standard estimate `count*(1+k) + (sum >> k)`. This is
/// O(1) arithmetic per `k` — no per-sample pass.
fn best_k(sum: u128, count: u64) -> (u32, u64) {
    let mut best_k = 0u32;
    let mut best = u64::MAX;
    for k in 0..=30u32 {
        let bits = count * (1 + k as u64) + (sum >> k) as u64;
        if bits < best {
            best = bits;
            best_k = k;
        }
    }
    (best_k, best)
}

/// Plan the partitioned-Rice coding of `residual` once: compute per-partition
/// zig-zag sums at the finest order in a single pass, then evaluate coarser
/// orders by merging adjacent partitions. `block` is `order + residual.len()`.
fn plan_rice(residual: &[i64], order: usize, block: usize) -> RicePlan {
    // Largest partition order with equal partitions and a non-empty partition 0.
    let mut finest = 0u32;
    for po in (0..=MAX_PARTITION_ORDER).rev() {
        let parts = 1usize << po;
        if block % parts == 0 && (block >> po) > order {
            finest = po;
            break;
        }
    }
    let fparts = 1usize << finest;
    let part_len = block >> finest;

    // Per fine-partition zig-zag sums and sample counts (single pass over data).
    let mut sums = vec![0u128; fparts];
    let mut counts = vec![0u64; fparts];
    let mut idx = 0usize;
    for p in 0..fparts {
        let count = if p == 0 { part_len - order } else { part_len };
        let mut s = 0u128;
        for &v in &residual[idx..idx + count] {
            s += zigzag(v) as u128;
        }
        sums[p] = s;
        counts[p] = count as u64;
        idx += count;
    }

    // Evaluate each order 0..=finest by merging adjacent fine partitions.
    let mut best = RicePlan {
        po: 0,
        params: Vec::new(),
        bits: u64::MAX,
    };
    for po in 0..=finest {
        let parts = 1usize << po;
        let group = fparts / parts;
        let mut bits = 4u64; // partition-order field
        let mut params = Vec::with_capacity(parts);
        for j in 0..parts {
            let mut s = 0u128;
            let mut c = 0u64;
            for g in 0..group {
                s += sums[j * group + g];
                c += counts[j * group + g];
            }
            let (k, b) = best_k(s, c);
            bits += 5 + b;
            params.push(k);
        }
        if bits < best.bits {
            best = RicePlan { po, params, bits };
        }
    }
    best
}

/// Write the residual coding (method 1: 5-bit Rice parameters).
fn write_residual(w: &mut BitWriter, residual: &[i64], order: usize, block: usize) {
    let plan = plan_rice(residual, order, block);
    let part_len = block >> plan.po;

    w.write(1, 2); // residual method 1 (5-bit Rice params)
    w.write(plan.po as u64, 4);

    let mut idx = 0usize;
    for (p, &k) in plan.params.iter().enumerate() {
        let count = if p == 0 { part_len - order } else { part_len };
        w.write(k as u64, 5);
        for &v in &residual[idx..idx + count] {
            let u = zigzag(v);
            w.write_unary((u >> k) as u32);
            if k > 0 {
                w.write(u & ((1 << k) - 1), k);
            }
        }
        idx += count;
    }
}

/// Write a FLAC/UTF-8-style coded number (frame or sample number).
fn write_utf8(w: &mut BitWriter, n: u64) {
    if n < 0x80 {
        w.write(n, 8);
        return;
    }
    let (bytes, _) = utf8_layout(n);
    let c = bytes;
    // Leading byte: `c` ones, a zero, then the top bits.
    let lead_ones = c as u32;
    let prefix = (0xFFu64 << (8 - lead_ones)) & 0xFF;
    let top = n >> (6 * (c - 1));
    w.write(prefix | top, 8);
    for i in (0..c - 1).rev() {
        let six = (n >> (6 * i)) & 0x3F;
        w.write(0x80 | six, 8);
    }
}

/// Number of bytes a UTF-8-coded `n` needs (2..=7).
fn utf8_layout(n: u64) -> (usize, u64) {
    let bytes = if n < 0x800 {
        2
    } else if n < 0x10000 {
        3
    } else if n < 0x20_0000 {
        4
    } else if n < 0x400_0000 {
        5
    } else if n < 0x8000_0000 {
        6
    } else {
        7
    };
    (bytes, n)
}
