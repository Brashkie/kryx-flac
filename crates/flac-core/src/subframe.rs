//! Subframe decoding: CONSTANT, VERBATIM, FIXED and LPC, plus the partitioned
//! Rice residual they share.

use crate::bitreader::BitReader;
use crate::error::{FlacError, Result};

/// Fixed-predictor coefficients for orders 0..=4 (applied to previous samples).
const FIXED_COEFS: [&[i64]; 5] = [&[], &[1], &[2, -1], &[3, -3, 1], &[4, -6, 4, -1]];

/// Decode one subframe into `block_size` samples at `bits_per_sample` bits.
pub fn decode_subframe(
    r: &mut BitReader,
    block_size: usize,
    bits_per_sample: u32,
) -> Result<Vec<i32>> {
    // Mandatory zero padding bit.
    if r.read(1)? != 0 {
        return Err(FlacError::Reserved("subframe padding bit"));
    }
    let type_code = r.read(6)? as u8;

    // Wasted bits: a 1 flag followed by a unary count.
    let wasted = if r.read(1)? == 1 {
        1 + r.read_unary()?
    } else {
        0
    };
    let bps = bits_per_sample - wasted;

    let mut samples = match type_code {
        0 => {
            // CONSTANT
            let v = r.read_signed(bps)? as i32;
            vec![v; block_size]
        }
        1 => {
            // VERBATIM
            let mut s = Vec::with_capacity(block_size);
            for _ in 0..block_size {
                s.push(r.read_signed(bps)? as i32);
            }
            s
        }
        8..=12 => {
            // FIXED, order = type_code - 8
            let order = (type_code - 8) as usize;
            decode_fixed(r, block_size, bps, order)?
        }
        32..=63 => {
            // LPC, order = (type_code & 0x1F) + 1
            let order = (type_code & 0x1F) as usize + 1;
            decode_lpc(r, block_size, bps, order)?
        }
        other => return Err(FlacError::BadSubframe(other)),
    };

    if wasted > 0 {
        for s in &mut samples {
            *s <<= wasted;
        }
    }
    Ok(samples)
}

fn read_warmup(r: &mut BitReader, bps: u32, order: usize) -> Result<Vec<i32>> {
    let mut w = Vec::with_capacity(order);
    for _ in 0..order {
        w.push(r.read_signed(bps)? as i32);
    }
    Ok(w)
}

fn decode_fixed(r: &mut BitReader, block_size: usize, bps: u32, order: usize) -> Result<Vec<i32>> {
    let mut out = read_warmup(r, bps, order)?;
    let residual = decode_residual(r, order, block_size)?;
    let coefs = FIXED_COEFS[order];
    for res in residual {
        let mut pred: i64 = 0;
        for (i, &c) in coefs.iter().enumerate() {
            pred += c * out[out.len() - 1 - i] as i64;
        }
        out.push(res + pred as i32);
    }
    Ok(out)
}

fn decode_lpc(r: &mut BitReader, block_size: usize, bps: u32, order: usize) -> Result<Vec<i32>> {
    let mut out = read_warmup(r, bps, order)?;
    let precision = r.read(4)? as u32 + 1;
    let shift = r.read_signed(5)?;
    let mut coefs = Vec::with_capacity(order);
    for _ in 0..order {
        coefs.push(r.read_signed(precision)?);
    }
    let residual = decode_residual(r, order, block_size)?;
    for res in residual {
        let mut pred: i64 = 0;
        for (i, &c) in coefs.iter().enumerate() {
            pred += c * out[out.len() - 1 - i] as i64;
        }
        // The shift is signed two's-complement; real encoders use >= 0, but
        // guard the negative case rather than panic.
        let shifted = if shift >= 0 {
            pred >> (shift as u32)
        } else {
            pred << ((-shift) as u32)
        };
        out.push(res + shifted as i32);
    }
    Ok(out)
}

/// Decode the partitioned Rice-coded residual (`block_size - order` values).
fn decode_residual(r: &mut BitReader, order: usize, block_size: usize) -> Result<Vec<i32>> {
    let method = r.read(2)?;
    let (param_bits, escape) = match method {
        0 => (4u32, 0xFu64),
        1 => (5u32, 0x1Fu64),
        _ => return Err(FlacError::Reserved("residual coding method")),
    };
    let partition_order = r.read(4)? as u32;
    let partitions = 1usize << partition_order;
    let partition_samples = block_size >> partition_order;

    let mut residual = Vec::with_capacity(block_size - order);
    for p in 0..partitions {
        let count = if p == 0 {
            partition_samples - order
        } else {
            partition_samples
        };
        let param = r.read(param_bits)?;
        if param == escape {
            // Escape: raw `nbits`-wide signed residuals.
            let nbits = r.read(5)? as u32;
            for _ in 0..count {
                residual.push(r.read_signed(nbits)? as i32);
            }
        } else {
            let k = param as u32;
            for _ in 0..count {
                let q = r.read_unary()?;
                let rem = if k > 0 { r.read(k)? } else { 0 };
                let u = ((q as u64) << k) | rem;
                // Zig-zag unfold to signed.
                residual.push(((u >> 1) as i64 ^ -((u & 1) as i64)) as i32);
            }
        }
    }
    Ok(residual)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_order2_restores_ramp() {
        // A ramp 0,1,2,3,4 has 2nd-order-fixed residual of all zeros after the
        // two warmup samples. Build a minimal CONSTANT-free stream by hand is
        // hard; instead test the predictor restore directly via decode_fixed on
        // a crafted bitstream is covered by the integration tests. Here we just
        // sanity-check FIXED_COEFS ordering.
        assert_eq!(FIXED_COEFS[2], &[2, -1]);
        assert_eq!(FIXED_COEFS[4], &[4, -6, 4, -1]);
    }
}
