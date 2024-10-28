/// Experimental CABAC using rANS encoder/decoder
///
/// The performance of this implementation is not yet optimized and may not ever be faster
/// than the VP8 implementation. This is a proof of concept.
use std::{
    collections::VecDeque,
    io::{Read, Write},
};

use bytemuck::cast_slice;
use std::io::Result;

use crate::{
    traits::{CabacReader, CabacWriter, GetInnerBuffer},
    vp8::VP8Context,
};

pub type RansContext = VP8Context;

#[derive(Clone, Copy)]
pub struct Rans64State<const SCALE_BITS: u32>(u64);

impl<const SCALE_BITS: u32> std::fmt::Debug for Rans64State<SCALE_BITS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

const RANS64_L: u64 = 1 << 31; // Lower bound of our normalization interval

pub trait WriteU32 {
    fn write_u32(&mut self, value: u32);
}

impl WriteU32 for VecDeque<u32> {
    fn write_u32(&mut self, value: u32) {
        self.push_front(u32::to_le(value));
    }
}

fn read_u32(r: &mut impl Read) -> Result<u32> {
    let mut b = [0; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

impl<const SCALE_BITS: u32> Rans64State<SCALE_BITS> {
    pub fn new_encoder() -> Self {
        Rans64State(RANS64_L)
    }

    // Initializes a rANS decoder.
    pub fn new_decoder(pptr: &mut impl Read) -> Result<Self> {
        Ok(Rans64State(
            u64::from(read_u32(pptr)?) | (u64::from(read_u32(pptr)?) << 32),
        ))
    }

    // Flushes the rANS encoder.
    pub fn enc_flush(&mut self, buffer: &mut impl WriteU32) {
        let x = self.0;

        buffer.write_u32((x >> 32) as u32);
        buffer.write_u32((x >> 0) as u32);
    }

    // Returns the current cumulative frequency.
    pub fn dec_get(&self) -> u32 {
        (self.0 & ((1u64 << SCALE_BITS) - 1)) as u32
    }

    // Advances in the bit stream by "popping" a single symbol.
    pub fn dec_advance(&mut self, r: &mut impl Read, start: u32, freq: u32) -> Result<()> {
        let mask = (1u64 << SCALE_BITS) - 1;

        // s, x = D(x)
        let mut x = self.0;
        x = u64::from(freq) * (x >> SCALE_BITS) + (x & mask) - u64::from(start);

        // Renormalize
        if x < RANS64_L {
            x = (x << 32) | u64::from(read_u32(r)?);
            assert!(x >= RANS64_L);
        }

        self.0 = x;
        Ok(())
    }

    // Encodes a single symbol
    pub fn encode(&mut self, output: &mut impl WriteU32, start: u32, freq: u32) {
        assert!(freq != 0);

        let mut x = self.0;
        let x_max = ((RANS64_L >> SCALE_BITS) << 32) * u64::from(freq);

        if x >= x_max {
            output.write_u32(x as u32);
            x >>= 32;
            assert!(x < x_max);
        }

        self.0 = (x / u64::from(freq)) << SCALE_BITS | (x % u64::from(freq)) + start as u64;
    }

    // Advances in the bit stream without output.
    pub fn dec_advance_step(&mut self, start: u32, freq: u32) {
        let mask = (1u64 << SCALE_BITS) - 1;

        let x = self.0;
        self.0 = freq as u64 * (x >> SCALE_BITS) + (x & mask) - start as u64;
    }
}

pub struct RansReader64<R> {
    pub rans: Rans64State<8>,
    pub upstream_reader: R,
}

impl<R: Read> RansReader64<R> {
    pub fn new(mut reader: R) -> Result<Self> {
        let rans = Rans64State::new_decoder(&mut reader)?;
        Ok(RansReader64 {
            rans: rans,
            upstream_reader: reader,
        })
    }
}

pub struct RansWriter64<W> {
    pub upstream_writer: W,
    pub symbol_buffer: Vec<(bool, u8)>,
}

impl GetInnerBuffer for RansWriter64<Vec<u8>> {
    fn inner_buffer(&self) -> &[u8] {
        &self.upstream_writer
    }
}

impl<W: Write> RansWriter64<W> {
    pub fn new(writer: W) -> Self {
        RansWriter64 {
            upstream_writer: writer,
            symbol_buffer: Vec::new(),
        }
    }
}

impl<W: Write> CabacWriter<RansContext> for RansWriter64<W> {
    fn put(&mut self, bit: bool, branch: &mut RansContext) -> Result<()> {
        let prob = branch.get_probability();
        self.symbol_buffer.push((bit, prob));
        branch.record_and_update_bit(bit);
        Ok(())
    }

    fn put_bypass(&mut self, bit: bool) -> Result<()> {
        self.symbol_buffer.push((bit, 128));
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        let mut rans = Rans64State::<8>::new_encoder();
        let mut write_buffer: VecDeque<u32> = VecDeque::new();

        for &(bit, prob) in self.symbol_buffer.iter().rev() {
            let (start, freq) = start_freq(bit, prob);
            rans.encode(&mut write_buffer, start, freq);
        }

        rans.enc_flush(&mut write_buffer);

        let slices = write_buffer.as_slices();

        self.upstream_writer.write_all(cast_slice(&slices.0))?;
        self.upstream_writer.write_all(cast_slice(&slices.1))?;
        Ok(())
    }
}

fn start_freq(bit: bool, prob: u8) -> (u32, u32) {
    if bit {
        (prob as u32, 256 - prob as u32)
    } else {
        (0, prob as u32)
    }
}

impl<R: Read> CabacReader<RansContext> for RansReader64<R> {
    fn get(&mut self, branch: &mut RansContext) -> Result<bool> {
        let prob = branch.get_probability();

        let cumulative_freq = self.rans.dec_get();

        let bit = cumulative_freq >= u32::from(prob);

        branch.record_and_update_bit(bit);
        let (start, freq) = start_freq(bit, prob);
        self.rans
            .dec_advance(&mut self.upstream_reader, start, freq)?;
        Ok(bit)
    }

    fn get_bypass(&mut self) -> Result<bool> {
        let prob = 128;

        let cumulative_freq = self.rans.dec_get();

        let bit = cumulative_freq >= u32::from(prob);

        let (start, freq) = start_freq(bit, prob);
        self.rans
            .dec_advance(&mut self.upstream_reader, start, freq)?;
        Ok(bit)
    }
}
