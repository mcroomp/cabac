/*
 *  Copyright (c) 2010 The WebM project authors. All Rights Reserved.
 *
 *  Use of this source code is governed by a BSD-style license
 *  that can be found in the LICENSE banner below
 *  An additional intellectual property rights grant can be found
 *  in the file PATENTS.  All contributing project authors may
 *  be found in the VPX_AUTHORS file in this directory
 */
/*
Copyright (c) 2010, Google Inc. All rights reserved.
Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:
Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.
Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.
Neither the name of Google nor the names of its contributors may be used to endorse or promote products derived from this software without specific prior written permission.
THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS “AS IS” AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
*/

use std::io::{Read, Result, Write};

use crate::cabac::{CabacReader, CabacWriter};

const BITS_IN_BYTE: i32 = 8;
const BITS_IN_LONG: i32 = 64;
const BITS_IN_LONG_MINUS_LAST_BYTE: i32 = BITS_IN_LONG - BITS_IN_BYTE;

pub struct VP8Context {
    counts: u16,
}

impl Default for VP8Context {
    fn default() -> Self {
        VP8Context { counts: 0x101 }
    }
}

// used to precalculate the probabilities
const fn problookup() -> [u8; 65536] {
    let mut retval = [0; 65536];
    let mut i = 1i32;
    while i < 65536 {
        let a = i >> 8;
        let b = i & 0xff;

        retval[i as usize] = ((a << 8) / (a + b)) as u8;
        i += 1;
    }

    return retval;
}

static PROB_LOOKUP: [u8; 65536] = problookup();

impl VP8Context {
    #[inline(always)]
    pub fn get_probability(&self) -> u8 {
        // 0 is a special corner case which should return probability 0
        // since 0 is impossible to happen since the counts always start at 1
        PROB_LOOKUP[self.counts as usize]
    }

    #[inline(always)]
    pub fn record_and_update_true_obs(&mut self) {
        if self.counts == 0 {
            return; // no need to do anything since we are already as baised towards all trues as possible
        }

        if (self.counts & 0xff) != 0xff {
            // non-overflow case is easy
            self.counts += 1;
        } else {
            // special case where it is all trues
            if self.counts == 0x01ff {
                // corner case since the original implementation
                // insists on setting the probabily to zero,
                // although the probability calculation would
                // return 1.
                self.counts = 0;
            } else {
                self.counts = (((self.counts as u32 + 0x100) >> 1) & 0xff00) as u16 | 129;
            }
        }
    }

    #[inline(always)]
    pub fn record_and_update_false_obs(&mut self) {
        if self.counts == 0 {
            // handle corner case where prob was set badly
            self.counts = 0x02ff;
            return;
        }

        if (self.counts & 0xff00) != 0xff00 {
            // non-overflow case is easy
            self.counts += 0x100;
        } else {
            // special case where it is all falses
            if self.counts == 0xff01 {
            } else {
                self.counts = ((1 + (self.counts & 0xff) as u32) >> 1) as u16 | 0x8100;
            }
        }
    }
}

pub struct VP8Reader<R> {
    value: u64,
    range: u32,
    count: i32,
    upstream_reader: R,
}

impl<R: Read> CabacReader<VP8Context> for VP8Reader<R> {
    fn get(&mut self, branch: &mut VP8Context) -> Result<bool> {
        if self.count < 0 {
            self.vpx_reader_fill()?;
        }

        let prob = branch.get_probability() as u32;

        let mut tmp_range = self.range;
        let mut tmp_value = self.value;

        let split = ((tmp_range * prob) + (256 - prob)) >> BITS_IN_BYTE;
        let big_split = (split as u64) << BITS_IN_LONG_MINUS_LAST_BYTE;
        let bit = tmp_value >= big_split;

        if bit {
            branch.record_and_update_true_obs();
            tmp_range = tmp_range - split;
            tmp_value -= big_split;
        } else {
            branch.record_and_update_false_obs();
            tmp_range = split;
        }

        //lookup tables are best avoided in modern CPUs
        //let shift = VPX_NORM[tmp_range as usize] as i32;
        let shift = (tmp_range as u8).leading_zeros() as i32;

        self.value = tmp_value << shift;
        self.count -= shift;
        self.range = tmp_range << shift;

        return Ok(bit);
    }

    fn get_bypass(&mut self) -> Result<bool> {
        if self.count < 0 {
            self.vpx_reader_fill()?;
        }

        let prob = 128;

        let mut tmp_range = self.range;
        let mut tmp_value = self.value;

        let split = ((tmp_range * prob) + (256 - prob)) >> BITS_IN_BYTE;
        let big_split = (split as u64) << BITS_IN_LONG_MINUS_LAST_BYTE;
        let bit = tmp_value >= big_split;

        if bit {
            tmp_range = tmp_range - split;
            tmp_value -= big_split;
        } else {
            tmp_range = split;
        }

        //lookup tables are best avoided in modern CPUs
        //let shift = VPX_NORM[tmp_range as usize] as i32;
        let shift = (tmp_range as u8).leading_zeros() as i32;

        self.value = tmp_value << shift;
        self.count -= shift;
        self.range = tmp_range << shift;

        return Ok(bit);
    }
}

impl<R: Read> VP8Reader<R> {
    pub fn new(reader: R) -> Result<Self> {
        let mut r = VP8Reader {
            upstream_reader: reader,
            value: 0,
            count: -8,
            range: 255,
        };

        r.vpx_reader_fill()?;

        let mut dummy_branch = VP8Context::default();
        r.get(&mut dummy_branch)?; // marker bit

        return Ok(r);
    }

    fn vpx_reader_fill(&mut self) -> Result<()> {
        let mut tmp_value = self.value;
        let mut tmp_count = self.count;
        let mut shift = BITS_IN_LONG_MINUS_LAST_BYTE - (tmp_count + BITS_IN_BYTE);

        while shift >= 0 {
            // BufReader is already pretty efficient handling small reads, so optimization doesn't help that much
            let mut v = [0u8; 1];
            let bytes_read = self.upstream_reader.read(&mut v[..])?;
            if bytes_read == 0 {
                break;
            }

            tmp_value |= (v[0] as u64) << shift;
            shift -= BITS_IN_BYTE;
            tmp_count += BITS_IN_BYTE;
        }

        self.value = tmp_value;
        self.count = tmp_count;

        return Ok(());
    }
}

pub struct VP8Writer<W> {
    low_value: u32,
    range: u32,
    count: i32,
    writer: W,
    buffer: Vec<u8>,
}

impl<W: Write> VP8Writer<W> {
    pub fn new(writer: W) -> Result<Self> {
        let mut retval = VP8Writer {
            low_value: 0,
            range: 255,
            count: -24,
            buffer: Vec::new(),
            writer: writer,
        };

        let mut dummy_branch = VP8Context::default();
        retval.put(false, &mut dummy_branch)?;

        Ok(retval)
    }

    /// When buffer is full and is going to be sent to output, preserve buffer data that
    /// is not final and should carried over to the next buffer.
    fn flush_non_final_data(&mut self) -> Result<()> {
        // carry over buffer data that might be not final
        let mut i = self.buffer.len() - 1;
        while self.buffer[i] == 0xFF {
            assert!(i > 0);
            i -= 1;
        }

        self.writer.write_all(&self.buffer[..i])?;
        self.buffer.drain(..i);

        Ok(())
    }
}

impl<W: Write> CabacWriter<VP8Context> for VP8Writer<W> {
    fn put(&mut self, value: bool, branch: &mut VP8Context) -> Result<()> {
        let probability = branch.get_probability() as u32;

        let mut tmp_range = self.range;
        let split = 1 + (((tmp_range - 1) * probability) >> 8);

        let mut tmp_low_value = self.low_value;
        if value {
            branch.record_and_update_true_obs();
            tmp_low_value += split;
            tmp_range -= split;
        } else {
            branch.record_and_update_false_obs();
            tmp_range = split;
        }

        //lookup tables are best avoided in modern CPUs
        //let mut shift = VPX_NORM[tmp_range as usize] as i32;
        let mut shift = tmp_range.leading_zeros() as i32 - 24;

        tmp_range <<= shift;

        let mut tmp_count = self.count;
        tmp_count += shift;

        if tmp_count >= 0 {
            let offset = shift - tmp_count;

            if ((tmp_low_value << (offset - 1)) & 0x80000000) != 0 {
                let mut x = self.buffer.len() - 1;

                while self.buffer[x] == 0xFF {
                    self.buffer[x] = 0;

                    assert!(x > 0);
                    x -= 1;
                }

                self.buffer[x] += 1;
            }

            self.buffer.push((tmp_low_value >> (24 - offset)) as u8);
            tmp_low_value <<= offset;
            shift = tmp_count;
            tmp_low_value &= 0xffffff;
            tmp_count -= 8;
        }

        tmp_low_value <<= shift;

        self.count = tmp_count;
        self.low_value = tmp_low_value;
        self.range = tmp_range;

        // check if we're out of buffer space, if yes - send the buffer to output,
        if self.buffer.len() > 65536 - 128 {
            self.flush_non_final_data()?;
        }

        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        for _i in 0..32 {
            let mut dummy_branch = VP8Context::default();
            self.put(false, &mut dummy_branch)?;
        }

        // Ensure there's no ambigous collision with any index marker bytes
        if (self.buffer.last().unwrap() & 0xe0) == 0xc0 {
            self.buffer.push(0);
        }

        self.writer.write_all(&self.buffer[..])?;

        Ok(())
    }

    fn put_bypass(&mut self, value: bool) -> Result<()> {
        let probability = 128;

        let mut tmp_range = self.range;
        let split = 1 + (((tmp_range - 1) * probability) >> 8);

        let mut tmp_low_value = self.low_value;
        if value {
            tmp_low_value += split;
            tmp_range -= split;
        } else {
            tmp_range = split;
        }

        //lookup tables are best avoided in modern CPUs
        //let mut shift = VPX_NORM[tmp_range as usize] as i32;
        let mut shift = tmp_range.leading_zeros() as i32 - 24;

        tmp_range <<= shift;

        let mut tmp_count = self.count;
        tmp_count += shift;

        if tmp_count >= 0 {
            let offset = shift - tmp_count;

            if ((tmp_low_value << (offset - 1)) & 0x80000000) != 0 {
                let mut x = self.buffer.len() - 1;

                while self.buffer[x] == 0xFF {
                    self.buffer[x] = 0;

                    assert!(x > 0);
                    x -= 1;
                }

                self.buffer[x] += 1;
            }

            self.buffer.push((tmp_low_value >> (24 - offset)) as u8);
            tmp_low_value <<= offset;
            shift = tmp_count;
            tmp_low_value &= 0xffffff;
            tmp_count -= 8;
        }

        tmp_low_value <<= shift;

        self.count = tmp_count;
        self.low_value = tmp_low_value;
        self.range = tmp_range;

        // check if we're out of buffer space, if yes - send the buffer to output,
        if self.buffer.len() > 65536 - 128 {
            self.flush_non_final_data()?;
        }

        Ok(())
    }
}
