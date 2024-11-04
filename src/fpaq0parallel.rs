use wide::u32x4;

/// Special version of FPaq0 that allows for parallel encoding and decoding. There is some overhead on the encoding
/// side since we need to track the future output byte locations so that the reader can read them back without any
/// special signalling.
///
/// Original algorithm developed by Matt Mahoney https://mattmahoney.net/dc/fpaq0.cpp
///
/// I like this implementation since it has no carry processing compared to other arithmetic encoders and the bytes
/// align exactly with reads and writes. This makes it especially suitable for this kind of parallel encoding and decoding.
///
/// As long as you exactly match your puts and gets, you can even put bytes in the middle of the stream, as long
/// as you read them back in the same order.
///
/// This gives you many of the advantages of rANS decoding without the need to do everything in reverse, and also
/// the encoding doesn't require any divide/mod ops like rANS does. It does require a bit more memory with the future
/// buffer, but as long as the use of the contexts is fairly balanced, it should be a good tradeoff. ie don't do this:
///
///  for i in 0..100000 {
///   context1.put(bit, &mut output);
///  }
///  context2.put(bit,&mut output);
///
/// Parallelization implements the idea from:
/// P. G. Howard, "Interleaving entropy codes," Proceedings. Compression and Complexity of SEQUENCES 1997
///  (Cat. No.97TB100171), Salerno, Italy, 1997, pp. 45-55, doi: 10.1109/SEQUEN.1997.666902.
use crate::vp8::VP8Context;
use std::{
    collections::VecDeque,
    io::{Read, Result, Write},
};

/// Decodes a byte stream encoded by Fpaq0EncoderParallel
pub struct Fpaq0DecoderParallel {
    xl: u32,
    xr: u32,
    x: u32,
}

impl std::fmt::Debug for Fpaq0DecoderParallel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Fpaq0Decoder {{ xl: {:x}, xr: {:x}, x: {:x} }}",
            self.xl, self.xr, self.x
        )
    }
}

impl Fpaq0DecoderParallel {
    pub fn new(reader: &mut impl Read) -> Result<Self> {
        let mut x: u32 = 0;
        for _ in 0..4 {
            let mut b = [0u8];
            let _ = reader.read_exact(&mut b)?;

            x = (x << 8) | u32::from(b[0]);
        }

        Ok(Fpaq0DecoderParallel {
            xl: 0,
            xr: 0xffff_ffff,
            x,
        })
    }

    fn fill_bits(&mut self, reader: &mut impl Read) -> Result<()> {
        while 0 == ((self.xl ^ self.xr) & 0xFF00_0000) {
            self.xl <<= 8;
            self.xr = (self.xr << 8) | 0x0000_00FF;

            let mut b = [0u8];
            let _ = reader.read_exact(&mut b)?;

            self.x = (self.x << 8) | u32::from(b[0]);
        }
        Ok(())
    }

    /// reads a bit from the stream given a certain probability context
    pub fn get(&mut self, cur_ctx: &mut VP8Context, reader: &mut impl Read) -> Result<bool> {
        let xm = self.xl + ((self.xr - self.xl) >> 8) * u32::from(cur_ctx.get_probability().get());
        let mut bit = true;
        if self.x <= xm {
            bit = false;
            self.xr = xm;
        } else {
            self.xl = xm + 1;
        }

        cur_ctx.record_and_update_bit(bit);

        self.fill_bits(reader)?;

        Ok(bit)
    }
}

/// Decodes a byte stream encoded by Fpaq0EncoderParallel
pub struct Fpaq0DecoderParallelSimd {
    xl: u32x4,
    xr: u32x4,
    x: u32x4,
}

impl std::fmt::Debug for Fpaq0DecoderParallelSimd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Fpaq0Decoder {{ xl: {:x}, xr: {:x}, x: {:x} }}",
            self.xl, self.xr, self.x
        )
    }
}

impl Fpaq0DecoderParallelSimd {
    pub fn new(reader: &mut impl Read) -> Result<Self> {
        let mut x = [0u32; 4];
        for i in 0..4 {
            for _ in 0..4 {
                let mut b = [0u8];
                let _ = reader.read_exact(&mut b)?;

                x[i] = (x[i] << 8) | u32::from(b[0]);
            }
        }

        Ok(Fpaq0DecoderParallelSimd {
            xl: u32x4::splat(0),
            xr: u32x4::splat(0xffff_ffff),
            x: u32x4::from(x),
        })
    }

    fn fill_bits(&mut self, reader: &mut impl Read, i: usize) -> Result<()> {
        while 0 == ((self.xl.as_array_ref()[i] ^ self.xr.as_array_ref()[i]) & 0xFF00_0000) {
            self.xl.as_array_mut()[i] <<= 8;
            self.xr.as_array_mut()[i] = (self.xr.as_array_ref()[i] << 8) | 0x0000_00FF;

            let mut b = [0u8];
            let _ = reader.read_exact(&mut b)?;

            self.x.as_array_mut()[i] = (self.x.as_array_ref()[i] << 8) | u32::from(b[0]);
        }
        Ok(())
    }

    /// reads a bit from the stream given a certain probability context
    pub fn get(
        &mut self,
        cur_ctx: &mut [VP8Context; 4],
        reader: &mut impl Read,
    ) -> Result<[bool; 4]> {
        let xm: u32x4 = self.xl
            + ((self.xr - self.xl) >> 8)
                * u32x4::from([
                    u32::from(cur_ctx[0].get_probability().get()),
                    u32::from(cur_ctx[1].get_probability().get()),
                    u32::from(cur_ctx[2].get_probability().get()),
                    u32::from(cur_ctx[3].get_probability().get()),
                ]);

        let mut bit = [true; 4];
        for i in 0..4 {
            if self.x.as_array_mut()[i] <= xm.as_array_ref()[i] {
                bit[i] = false;
                self.xr.as_array_mut()[i] = xm.as_array_ref()[i];
            } else {
                self.xl.as_array_mut()[i] = xm.as_array_ref()[i] + 1;
            }

            cur_ctx[i].record_and_update_bit(bit[i]);

            self.fill_bits(reader, i)?;
        }

        Ok(bit)
    }
}

#[derive(Clone, PartialEq, Eq)]
enum FutureOutputType {
    Reserved(u8),
    Commit(u8),
}

/// This holds the output and stitches together the future output in the right
/// order so that the reader can read it back without any special signalling.
pub struct EncoderOutput<W> {
    future_output: VecDeque<FutureOutputType>,
    output: W,
}

impl<W: Write> EncoderOutput<W> {
    pub fn new(output: W) -> Self {
        EncoderOutput {
            future_output: VecDeque::new(),
            output,
        }
    }

    /// writes a byte to the output stream in such a position that it can be
    /// read back by the decoder without any special signalling as long
    /// as it is done in the same order as it was written
    pub fn write_bypass_byte(&mut self, byte: u8) -> Result<()> {
        self.future_output.push_back(FutureOutputType::Commit(byte));

        self.write_ready_bytes()?;

        Ok(())
    }

    fn write_ready_bytes(&mut self) -> Result<()> {
        while let Some(FutureOutputType::Commit(v)) = self.future_output.front() {
            self.output.write_all(&[*v])?;
            let _ = self.future_output.pop_front().unwrap();
        }

        Ok(())
    }
}

pub struct Fpaq0EncoderParallel {
    xl: u32,
    xr: u32,
    id: u8,
}

impl std::fmt::Debug for Fpaq0EncoderParallel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fpaq0Encoder {{ xl: {:x}, xr: {:x} }}", self.xl, self.xr)
    }
}

impl Fpaq0EncoderParallel {
    pub fn new<W: Write>(output: &mut EncoderOutput<W>, id: u8) -> Self {
        for _i in 0..4 {
            output
                .future_output
                .push_back(FutureOutputType::Reserved(id));
        }

        Fpaq0EncoderParallel {
            xl: 0,
            xr: 0xffff_ffff,
            id,
        }
    }

    /// writes a byte to the steam in its reserved location. If repush is true, it will
    /// reserve a new location for the next byte.
    fn flush_byte<W: Write>(
        &mut self,
        byte: u8,
        output: &mut EncoderOutput<W>,
        repush: bool,
    ) -> Result<()> {
        for x in output.future_output.iter_mut() {
            if *x == FutureOutputType::Reserved(self.id) {
                *x = FutureOutputType::Commit(byte);
                break;
            }
        }
        if repush {
            output
                .future_output
                .push_back(FutureOutputType::Reserved(self.id));
        }

        // empty out everything that is ready to be written
        output.write_ready_bytes()?;

        Ok(())
    }

    fn flush_bits<W: Write>(&mut self, writer: &mut EncoderOutput<W>) -> Result<()> {
        while 0 == ((self.xl ^ self.xr) & 0xFF00_0000) {
            let byte = (self.xr >> 24) as u8;

            self.flush_byte(byte, writer, true)?;

            self.xl <<= 8;
            self.xr = (self.xr << 8) | 0x0000_00FF;
        }
        Ok(())
    }

    pub fn put<W: Write>(
        &mut self,
        bit: bool,
        branch: &mut VP8Context,
        writer: &mut EncoderOutput<W>,
    ) -> Result<()> {
        let xm = self.xl + ((self.xr - self.xl) >> 8) * u32::from(branch.get_probability().get());

        // left/lower part of the interval corresponds to zero

        if !bit {
            self.xr = xm;
        } else {
            self.xl = xm + 1;
        }

        branch.record_and_update_bit(bit);

        self.flush_bits(writer)
    }

    pub fn finish<W: Write>(&mut self, writer: &mut EncoderOutput<W>) -> Result<()> {
        let mut byte = (self.xr >> 24) as u8;

        for _ in 0..4 {
            self.flush_byte(byte, writer, false)?;
            byte = 0;
        }

        Ok(())
    }
}

#[test]
fn bypass_byte() {
    use byteorder::ReadBytesExt;

    let mut output = EncoderOutput {
        future_output: VecDeque::new(),
        output: Vec::new(),
    };

    {
        let mut context = VP8Context::default();

        let mut encoder = Fpaq0EncoderParallel::new(&mut output, 0);
        for i in 0i32..1024 {
            if i > 10 && i < 20 {
                output.write_bypass_byte(i as u8).unwrap();
            }
            encoder
                .put((i % 47) != 0, &mut context, &mut output)
                .unwrap();
        }

        encoder.finish(&mut output).unwrap();
        assert!(output.future_output.is_empty());
    }

    {
        let mut context = VP8Context::default();
        let mut reader = std::io::Cursor::new(&output.output);

        let mut decoder = Fpaq0DecoderParallel::new(&mut reader).unwrap();
        for i in 0..1024 {
            if i > 10 && i < 20 {
                assert_eq!(reader.read_u8().unwrap(), i as u8);
            }
            assert_eq!(
                decoder.get(&mut context, &mut reader).unwrap(),
                (i % 47) != 0
            );
        }
    }
}

#[test]
fn bypass_dual() {
    let mut output = EncoderOutput {
        future_output: VecDeque::new(),
        output: Vec::new(),
    };
    {
        let mut context1 = VP8Context::default();
        let mut context2 = VP8Context::default();
        let mut context3 = VP8Context::default();

        let mut encoder1 = Fpaq0EncoderParallel::new(&mut output, 0);
        let mut encoder2 = Fpaq0EncoderParallel::new(&mut output, 1);
        let mut encoder3 = Fpaq0EncoderParallel::new(&mut output, 2);
        for i in 0i32..1024 {
            encoder1
                .put((i % 47) != 0, &mut context1, &mut output)
                .unwrap();
            encoder2
                .put(i % 3 != 0, &mut context2, &mut output)
                .unwrap();
            encoder3
                .put(i % 5 != 0, &mut context3, &mut output)
                .unwrap();
        }

        encoder1.finish(&mut output).unwrap();
        encoder2.finish(&mut output).unwrap();
        encoder3.finish(&mut output).unwrap();

        // nothing should be left to write
        assert!(output.future_output.is_empty());
    }

    {
        let mut context1 = VP8Context::default();
        let mut context2 = VP8Context::default();
        let mut context3 = VP8Context::default();

        let mut reader = std::io::Cursor::new(&output.output);

        let mut decoder1 = Fpaq0DecoderParallel::new(&mut reader).unwrap();
        let mut decoder2 = Fpaq0DecoderParallel::new(&mut reader).unwrap();
        let mut decoder3 = Fpaq0DecoderParallel::new(&mut reader).unwrap();
        for i in 0..1024 {
            assert_eq!(
                decoder1.get(&mut context1, &mut reader).unwrap(),
                (i % 47) != 0
            );
            assert_eq!(
                decoder2.get(&mut context2, &mut reader).unwrap(),
                (i % 3) != 0
            );
            assert_eq!(
                decoder3.get(&mut context3, &mut reader).unwrap(),
                (i % 5) != 0
            );
        }
    }
}

#[test]
fn simd_test() {
    let mut output = EncoderOutput {
        future_output: VecDeque::new(),
        output: Vec::new(),
    };
    {
        let mut context = [
            VP8Context::default(),
            VP8Context::default(),
            VP8Context::default(),
            VP8Context::default(),
        ];
        let mut encoders = [
            Fpaq0EncoderParallel::new(&mut output, 0),
            Fpaq0EncoderParallel::new(&mut output, 1),
            Fpaq0EncoderParallel::new(&mut output, 2),
            Fpaq0EncoderParallel::new(&mut output, 3),
        ];

        for i in 0i32..1024 {
            encoders[0]
                .put((i % 47) != 0, &mut context[0], &mut output)
                .unwrap();
            encoders[1]
                .put(i % 3 != 0, &mut context[1], &mut output)
                .unwrap();
            encoders[2]
                .put(i % 5 != 0, &mut context[2], &mut output)
                .unwrap();
            encoders[3]
                .put(i % 7 != 0, &mut context[3], &mut output)
                .unwrap();
        }

        for i in 0..4 {
            encoders[i].finish(&mut output).unwrap();
        }

        // nothing should be left to write
        assert!(output.future_output.is_empty());
    }

    {
        let mut context = [
            VP8Context::default(),
            VP8Context::default(),
            VP8Context::default(),
            VP8Context::default(),
        ];

        let mut reader = std::io::Cursor::new(&output.output);

        let mut decoder = Fpaq0DecoderParallelSimd::new(&mut reader).unwrap();

        for i in 0..1024 {
            let bits = decoder.get(&mut context, &mut reader).unwrap();

            assert_eq!(bits[0], (i % 47) != 0);
            assert_eq!(bits[1], (i % 3) != 0);
            assert_eq!(bits[2], (i % 5) != 0);
            assert_eq!(bits[3], (i % 7) != 0);
        }
    }
}
