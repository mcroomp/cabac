use crate::{
    traits::{CabacReader, CabacWriter, GetInnerBuffer},
    vp8::VP8Context,
};
use std::io::{Read, Result, Write};

pub struct Fpaq0Decoder<R> {
    inner_reader: R,
    xl: u32,
    xr: u32,
    x: u32,
}

impl<R: Read> Fpaq0Decoder<R> {
    pub fn new(mut reader: R) -> Result<Self> {
        let mut x: u32 = 0;
        for _ in 0..4 {
            let mut b = [0u8];
            let _ = reader.read(&mut b)?;

            x = (x << 8) | u32::from(b[0]);
        }

        Ok(Fpaq0Decoder {
            inner_reader: reader,
            xl: 0,
            xr: 0xffff_ffff,
            x,
        })
    }
}

impl<R: Read> CabacReader<VP8Context> for Fpaq0Decoder<R> {
    fn get_bypass(&mut self) -> Result<bool> {
        self.get(&mut VP8Context::default())
    }

    fn get(&mut self, cur_ctx: &mut VP8Context) -> Result<bool> {
        let xm = self.xl + ((self.xr - self.xl) >> 8) * u32::from(cur_ctx.get_probability().get());
        let mut bit = true;
        if self.x <= xm {
            bit = false;
            self.xr = xm;
        } else {
            self.xl = xm + 1;
        }

        cur_ctx.record_and_update_bit(bit);

        while 0 == ((self.xl ^ self.xr) & 0xFF00_0000) {
            self.xl <<= 8;
            self.xr = (self.xr << 8) | 0x0000_00FF;

            let mut b = [0u8];
            let _ = self.inner_reader.read(&mut b)?;

            self.x = (self.x << 8) | u32::from(b[0]);
        }

        Ok(bit)
    }
}

pub struct Fpaq0Encoder<W> {
    inner_writer: W,
    xl: u32,
    xr: u32,
}

impl GetInnerBuffer for Fpaq0Encoder<Vec<u8>> {
    fn inner_buffer(&self) -> &[u8] {
        &self.inner_writer
    }
}

impl<W: Write> Fpaq0Encoder<W> {
    pub fn new(writer: W) -> Self {
        Fpaq0Encoder {
            inner_writer: writer,
            xl: 0,
            xr: 0xffff_ffff,
        }
    }

    fn flush_bits(&mut self) -> Result<()> {
        while 0 == ((self.xl ^ self.xr) & 0xFF00_0000) {
            let byte = (self.xr >> 24) as u8;
            self.inner_writer.write_all(&[byte])?;
            self.xl <<= 8;
            self.xr = (self.xr << 8) | 0x0000_00FF;
        }
        Ok(())
    }
}

impl<W: Write> CabacWriter<VP8Context> for Fpaq0Encoder<W> {
    fn put(&mut self, bit: bool, branch: &mut VP8Context) -> Result<()> {
        let xm = self.xl + ((self.xr - self.xl) >> 8) * u32::from(branch.get_probability().get());

        // left/lower part of the interval corresponds to zero

        if !bit {
            self.xr = xm;
        } else {
            self.xl = xm + 1;
        }

        branch.record_and_update_bit(bit);

        self.flush_bits()
    }

    fn put_bypass(&mut self, bit: bool) -> Result<()> {
        self.put(bit, &mut VP8Context::default())
    }

    fn finish(&mut self) -> Result<()> {
        let byte = (self.xr >> 24) as u8;
        self.inner_writer.write_all(&[byte])
    }
}
