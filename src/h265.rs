///! implementation of CABAC from H.264/H.265 codec. Uses a 6 bit state to track probabilities.
///
/*
 * H.265 video codec.
 * Copyright (c) 2013-2014 struktur AG, Dirk Farin <farin@struktur.de>
 *
 * This file is part of libde265.
 *
 * libde265 is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Lesser General Public License as
 * published by the Free Software Foundation, either version 3 of
 * the License, or (at your option) any later version.
 *
 * libde265 is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Lesser General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General Public License
 * along with libde265.  If not, see <http://www.gnu.org/licenses/>.
 */
use std::io::{Read, Result, Write};

use byteorder::{ReadBytesExt, WriteBytesExt};

use crate::traits::{CabacReader, CabacWriter};

const NEXT_STATE_MPS: [u8; 128] = [
    2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51,
    52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75,
    76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99,
    100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118,
    119, 120, 121, 122, 123, 124, 125, 124, 125, 126, 127,
];

const NEXT_STATE_LPS: [u8; 128] = [
    1, 0, 0, 1, 2, 3, 4, 5, 4, 5, 8, 9, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 18, 19, 22,
    23, 22, 23, 24, 25, 26, 27, 26, 27, 30, 31, 30, 31, 32, 33, 32, 33, 36, 37, 36, 37, 38, 39, 38,
    39, 42, 43, 42, 43, 44, 45, 44, 45, 46, 47, 48, 49, 48, 49, 50, 51, 52, 53, 52, 53, 54, 55, 54,
    55, 56, 57, 58, 59, 58, 59, 60, 61, 60, 61, 60, 61, 62, 63, 64, 65, 64, 65, 66, 67, 66, 67, 66,
    67, 68, 69, 68, 69, 70, 71, 70, 71, 70, 71, 72, 73, 72, 73, 72, 73, 74, 75, 74, 75, 74, 75, 76,
    77, 76, 77, 126, 127,
];

const LPST_TABLE: [[u8; 4]; 64] = [
    [128, 176, 208, 240],
    [128, 167, 197, 227],
    [128, 158, 187, 216],
    [123, 150, 178, 205],
    [116, 142, 169, 195],
    [111, 135, 160, 185],
    [105, 128, 152, 175],
    [100, 122, 144, 166],
    [95, 116, 137, 158],
    [90, 110, 130, 150],
    [85, 104, 123, 142],
    [81, 99, 117, 135],
    [77, 94, 111, 128],
    [73, 89, 105, 122],
    [69, 85, 100, 116],
    [66, 80, 95, 110],
    [62, 76, 90, 104],
    [59, 72, 86, 99],
    [56, 69, 81, 94],
    [53, 65, 77, 89],
    [51, 62, 73, 85],
    [48, 59, 69, 80],
    [46, 56, 66, 76],
    [43, 53, 63, 72],
    [41, 50, 59, 69],
    [39, 48, 56, 65],
    [37, 45, 54, 62],
    [35, 43, 51, 59],
    [33, 41, 48, 56],
    [32, 39, 46, 53],
    [30, 37, 43, 50],
    [29, 35, 41, 48],
    [27, 33, 39, 45],
    [26, 31, 37, 43],
    [24, 30, 35, 41],
    [23, 28, 33, 39],
    [22, 27, 32, 37],
    [21, 26, 30, 35],
    [20, 24, 29, 33],
    [19, 23, 27, 31],
    [18, 22, 26, 30],
    [17, 21, 25, 28],
    [16, 20, 23, 27],
    [15, 19, 22, 25],
    [14, 18, 21, 24],
    [14, 17, 20, 23],
    [13, 16, 19, 22],
    [12, 15, 18, 21],
    [12, 14, 17, 20],
    [11, 14, 16, 19],
    [11, 13, 15, 18],
    [10, 12, 15, 17],
    [10, 12, 14, 16],
    [9, 11, 13, 15],
    [9, 11, 12, 14],
    [8, 10, 12, 14],
    [8, 9, 11, 13],
    [7, 9, 11, 12],
    [7, 9, 10, 12],
    [7, 8, 10, 11],
    [6, 8, 9, 11],
    [6, 7, 9, 10],
    [6, 7, 8, 9],
    [2, 2, 2, 2],
];

const RENORM_TABLE: [u8; 32] = [
    6, 5, 4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
];

/// context that tracks the probability of the next most probable symbol (either 1 or 0). Uses 6 bits.
#[derive(Default, Copy, Clone, Eq, PartialEq)]
pub struct H265Context {
    uc_state: u8,
}

impl H265Context {
    fn get_state(&self) -> u8 {
        self.uc_state >> 1
    }

    fn get_mps(&self) -> bool {
        (self.uc_state & 1) == 1
    }
    fn update_lps(&mut self) {
        self.uc_state = NEXT_STATE_LPS[usize::from(self.uc_state)];
    }
    fn update_mps(&mut self) {
        self.uc_state = NEXT_STATE_MPS[usize::from(self.uc_state)];
    }
}

/// CABAC encoder from H264/H265
pub struct H265Writer<W> {
    writer: W,
    low: u32,
    range: u32,
    buffered_byte: u32,
    num_buffered_bytes: i32,
    bits_left: i32,
}

impl<W: Write> CabacWriter<H265Context> for H265Writer<W> {
    fn put_bypass(&mut self, value: bool) -> Result<()> {
        self.low <<= 1;
        if value {
            self.low += self.range;
        }

        self.bits_left -= 1;

        if self.bits_left < 12 {
            self.flush_completed()?;
        }

        Ok(())
    }

    fn put(&mut self, value: bool, cur_ctx: &mut H265Context) -> Result<()> {
        let lps = LPST_TABLE[usize::from(cur_ctx.get_state())][((self.range >> 6) & 3) as usize];

        self.range -= u32::from(lps);

        if value != cur_ctx.get_mps() {
            let num_bits = RENORM_TABLE[usize::from(lps >> 3)];
            self.low = (self.low + self.range) << num_bits;
            self.range = u32::from(lps) << num_bits;

            cur_ctx.update_lps();

            self.bits_left -= i32::from(num_bits);
        } else {
            cur_ctx.update_mps();

            // renorm

            if self.range >= 256 {
                return Ok(());
            }

            self.low <<= 1;
            self.range <<= 1;
            self.bits_left -= 1;
        }

        if self.bits_left < 12 {
            self.flush_completed()?;
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        assert!(self.bits_left <= 32);

        if (self.low >> (32 - self.bits_left)) != 0 {
            self.writer.write_u8((self.buffered_byte + 1) as u8)?;
            while self.num_buffered_bytes > 1 {
                self.writer.write_u8(0x00)?;
                self.num_buffered_bytes -= 1;
            }

            self.low -= 1 << (32 - self.bits_left);
        } else {
            if self.num_buffered_bytes > 0 {
                self.writer.write_u8(self.buffered_byte as u8)?;
            }

            while self.num_buffered_bytes > 1 {
                self.writer.write_u8(0xff)?;
                self.num_buffered_bytes -= 1;
            }
        }

        // libde256 skips the last 8 bits, but this causes the last read to fail in some cases. It does
        // append a 1 bit to the end of the stream, but that still leaves some cases where the last
        // symbol will fail to decode properly.
        let mut bits = 32 - self.bits_left;

        let data = self.low;

        while bits >= 8 {
            self.writer.write_u8((data >> (bits - 8)) as u8)?;
            bits -= 8;
        }

        if bits > 0 {
            self.writer.write_u8((data << (8 - bits)) as u8)?;
        }

        Ok(())
    }
}

impl<W: Write> H265Writer<W> {
    pub fn new(writer: W) -> Self {
        H265Writer {
            writer,
            low: 0,
            range: 510,
            bits_left: 23,
            num_buffered_bytes: 0,
            buffered_byte: 0xff,
        }
    }

    fn flush_completed(&mut self) -> Result<()> {
        let lead_byte = self.low >> (24 - self.bits_left);
        self.bits_left += 8;
        self.low &= 0xffffffff >> self.bits_left;

        if lead_byte == 0xff {
            self.num_buffered_bytes += 1;
        } else if self.num_buffered_bytes > 0 {
            let carry = lead_byte >> 8;
            let mut byte = self.buffered_byte + carry;
            self.buffered_byte = lead_byte & 0xff;

            self.writer.write_u8(byte as u8)?;

            byte = (0xff + carry) & 0xff;
            while self.num_buffered_bytes > 1 {
                self.writer.write_u8(byte as u8)?;
                self.num_buffered_bytes -= 1;
            }
        } else {
            self.num_buffered_bytes = 1;
            self.buffered_byte = lead_byte;
        }
        Ok(())
    }
}

/// CABAC decoder from H265/H265
pub struct H265Reader<R> {
    reader: R,
    value: u32,
    range: u32,
    bits_needed: i32,
}

impl<R: Read> CabacReader<H265Context> for H265Reader<R> {
    fn get_bypass(&mut self) -> Result<bool> {
        self.value <<= 1;
        self.bits_needed += 1;

        if self.bits_needed >= 0 {
            self.bits_needed = -8;
            self.value |= u32::from(self.reader.read_u8()?);
        }

        let scaled_range = self.range << 7;

        let r = self.value.overflowing_sub(scaled_range);

        if r.1 {
            Ok(false)
        } else {
            self.value = r.0;
            Ok(true)
        }
    }

    fn get(&mut self, cur_ctx: &mut H265Context) -> Result<bool> {
        let mut range = self.range;
        let mut value = self.value;

        let lps = LPST_TABLE[usize::from(cur_ctx.get_state())][((range >> 6) & 3) as usize];

        range -= u32::from(lps);

        let scaled_range = range << 7;

        let bit;

        let r = value.overflowing_sub(scaled_range);

        if r.1 {
            // MPS path

            bit = cur_ctx.get_mps();

            cur_ctx.update_mps();

            if scaled_range < (256 << 7) {
                // scaled range, highest bit (15) not set

                range = scaled_range >> 6; // shift range by one bit
                value <<= 1; // shift value by one bit
                self.bits_needed += 1;

                if self.bits_needed == 0 {
                    self.bits_needed = -8;
                    value |= u32::from(self.reader.read_u8()?);
                }
            }
        } else {
            // LPS path

            value = r.0;

            let num_bits = RENORM_TABLE[usize::from(lps >> 3)];
            value <<= num_bits;
            range = u32::from(lps) << num_bits; /* this is always >= 0x100 except for state 63,
                                                but state 63 is never used */

            bit = !cur_ctx.get_mps();

            cur_ctx.update_lps();

            self.bits_needed += i32::from(num_bits);

            if self.bits_needed >= 0 {
                value |= u32::from(self.reader.read_u8()?) << self.bits_needed;

                self.bits_needed -= 8;
            }
        }

        self.range = range;
        self.value = value;

        Ok(bit)
    }
}

impl<R: Read> H265Reader<R> {
    pub fn new(reader: R) -> Result<Self> {
        let mut r = H265Reader {
            reader: reader,
            value: 0,
            range: 510,
            bits_needed: 8,
        };

        r.value = (u32::from(r.reader.read_u8()?) << 8) | u32::from(r.reader.read_u8()?);
        r.bits_needed -= 16;

        Ok(r)
    }
}
