use std::{
    collections::VecDeque,
    io::{Read, Result, Write},
};

use bytemuck::cast_slice;

use crate::{
    rans64::RansContext,
    traits::{CabacReader, CabacWriter, GetInnerBuffer},
};

pub trait WriteU16 {
    fn write_u16(&mut self, value: u16);
}

impl WriteU16 for VecDeque<u16> {
    fn write_u16(&mut self, value: u16) {
        self.push_front(u16::to_le(value));
    }
}

fn read_u16(r: &mut impl Read) -> Result<u16> {
    let mut b = [0; 2];
    r.read_exact(&mut b)?;
    Ok(u16::from_le_bytes(b))
}

#[derive(Clone, Copy)]
pub struct Rans32State<const SCALE_BITS: u32>(u32);

impl<const SCALE_BITS: u32> std::fmt::Debug for Rans32State<SCALE_BITS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

const RANS_WORD_L: u32 = 1 << 16; // Lower bound of our normalization interval

impl<const SCALE_BITS: u32> Rans32State<SCALE_BITS> {
    pub fn new_encoder() -> Self {
        Rans32State(RANS_WORD_L)
    }

    /// Initializes a rANS decoder.
    pub fn new_decoder(pptr: &mut impl Read) -> Result<Self> {
        Ok(Rans32State(
            u32::from(read_u16(pptr)?) | (u32::from(read_u16(pptr)?) << 16),
        ))
    }

    /// Flushes the rANS encoder.
    pub fn enc_flush(&mut self, buffer: &mut impl WriteU16) {
        let x = self.0;

        buffer.write_u16((x >> 16) as u16);
        buffer.write_u16((x >> 0) as u16);
    }

    // Returns the current cumulative frequency.
    pub fn dec_get(&self) -> u32 {
        (self.0 & ((1u32 << SCALE_BITS) - 1)) as u32
    }

    // Advances in the bit stream by "popping" a single symbol.
    pub fn dec_advance(&mut self, r: &mut impl Read, start: u32, freq: u32) -> Result<()> {
        let mask = (1u32 << SCALE_BITS) - 1;

        // s, x = D(x)
        let mut x = self.0;
        x = freq * (x >> SCALE_BITS) + (x & mask) - start;

        // Renormalize
        if x < RANS_WORD_L {
            x = (x << 16) | u32::from(read_u16(r)?);
            assert!(x >= RANS_WORD_L);
        }

        self.0 = x;
        Ok(())
    }

    // Encodes a single symbol
    pub fn encode(&mut self, output: &mut impl WriteU16, start: u32, freq: u32) {
        assert!(freq != 0);

        let mut x = self.0;
        let x_max = ((RANS_WORD_L >> SCALE_BITS) << 16) * u32::from(freq);

        if x >= x_max {
            output.write_u16(x as u16);
            x >>= 16;
            assert!(x < x_max);
        }

        self.0 = (x / freq) << SCALE_BITS | (x % freq) + start;
    }

    // Advances in the bit stream without output.
    pub fn dec_advance_step(&mut self, start: u32, freq: u32) {
        let mask = (1u32 << SCALE_BITS) - 1;

        let x = self.0;
        self.0 = freq * (x >> SCALE_BITS) + (x & mask) - start;
    }
}

pub struct RansReader32<R> {
    pub rans: Rans32State<8>,
    pub upstream_reader: R,
}

impl<R: Read> RansReader32<R> {
    pub fn new(mut reader: R) -> Result<Self> {
        let rans = Rans32State::new_decoder(&mut reader)?;
        Ok(RansReader32 {
            rans: rans,
            upstream_reader: reader,
        })
    }
}

pub struct RansWriter32<W> {
    pub upstream_writer: W,
    pub symbol_buffer: Vec<(bool, u8)>,
}

impl GetInnerBuffer for RansWriter32<Vec<u8>> {
    fn inner_buffer(&self) -> &[u8] {
        &self.upstream_writer
    }
}

impl<W: Write> RansWriter32<W> {
    pub fn new(writer: W) -> Self {
        RansWriter32 {
            upstream_writer: writer,
            symbol_buffer: Vec::new(),
        }
    }
}

impl<W: Write> CabacWriter<RansContext> for RansWriter32<W> {
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
        let mut rans = Rans32State::<8>::new_encoder();
        let mut write_buffer: VecDeque<u16> = VecDeque::new();

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

impl<R: Read> CabacReader<RansContext> for RansReader32<R> {
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
