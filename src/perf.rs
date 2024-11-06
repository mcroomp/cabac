use std::io::Cursor;

#[cfg(feature = "simd")]
use crate::fpaq0parallel::Fpaq0DecoderParallelSimd;

use crate::{
    fpaq0::{Fpaq0Decoder, Fpaq0Encoder},
    fpaq0parallel::{EncoderOutput, Fpaq0DecoderParallel, Fpaq0EncoderParallel},
    h265::{H265Reader, H265Writer},
    rans32::{RansReader32, RansWriter32},
    traits::{CabacReader, CabacWriter, GetInnerBuffer},
    vp8::{VP8Context, VP8Reader, VP8Writer},
};

#[inline(always)]
fn generic_put_pattern<C: Default, CW: CabacWriter<C> + GetInnerBuffer, FW: FnOnce() -> CW>(
    bypass: bool,
    pattern: &[bool],
    f: FW,
) -> Vec<u8> {
    let mut context = C::default();

    let mut writer = f();

    if bypass {
        for &b in pattern.iter() {
            writer.put_bypass(b).unwrap();
        }
    } else {
        for &b in pattern.iter() {
            writer.put(b, &mut context).unwrap();
        }
    }

    writer.finish().unwrap();
    writer.inner_buffer().to_vec()
}

#[inline(always)]
fn generic_get_pattern<'a, C: Default, CR: CabacReader<C>, FR: FnOnce(&'a [u8]) -> CR>(
    bypass: bool,
    pattern: &[bool],
    source: &'a [u8],
    f: FR,
) -> Box<[bool]> {
    let mut context = C::default();

    let mut output = vec![false; pattern.len()].into_boxed_slice();

    let mut reader = f(source);

    assert!(output.len() == pattern.len());
    if bypass {
        for i in 0..pattern.len() {
            output[i] = reader.get_bypass().unwrap();
        }
    } else {
        for i in 0..pattern.len() {
            output[i] = reader.get(&mut context).unwrap();
        }
    }

    output
}

#[cfg(test)]
fn generic_test_pattern(get: fn(&[bool], &[u8]) -> Box<[bool]>, put: fn(&[bool]) -> Vec<u8>) {
    let mut pattern = Vec::new();
    rand::Rng::sample_iter(rand::thread_rng(), &rand::distributions::Standard)
        .take(200)
        .for_each(|x| pattern.push(x));

    let encoded = put(&pattern);
    let decoded = get(&pattern, &encoded);

    assert!(pattern == &decoded[..]);
}

// rans32
#[inline(never)]
#[allow(dead_code)]
pub fn rans32_put_pattern(pattern: &[bool]) -> Vec<u8> {
    generic_put_pattern(false, pattern, || RansWriter32::new(Vec::new()))
}

#[inline(never)]
#[allow(dead_code)]
pub fn rans32_get_pattern(pattern: &[bool], source: &[u8]) -> Box<[bool]> {
    generic_get_pattern(false, pattern, &source, |vec| {
        RansReader32::new(Cursor::new(vec)).unwrap()
    })
}

#[inline(never)]
#[allow(dead_code)]
pub fn rans32_put_pattern_bypass(pattern: &[bool]) -> Vec<u8> {
    generic_put_pattern(true, pattern, || RansWriter32::new(Vec::new()))
}

#[inline(never)]
#[allow(dead_code)]
pub fn rans32_get_pattern_bypass(pattern: &[bool], source: &[u8]) -> Box<[bool]> {
    generic_get_pattern(true, pattern, &source, |vec| {
        RansReader32::new(Cursor::new(vec)).unwrap()
    })
}

#[test]
fn rans32_test_pattern() {
    generic_test_pattern(rans32_get_pattern, rans32_put_pattern);
    generic_test_pattern(rans32_get_pattern_bypass, rans32_put_pattern_bypass);
}

#[inline(never)]
#[allow(dead_code)]
pub fn vp8_put_pattern(pattern: &[bool]) -> Vec<u8> {
    generic_put_pattern(false, pattern, || VP8Writer::new(Vec::new()).unwrap())
}

#[inline(never)]
#[allow(dead_code)]
pub fn vp8_get_pattern(pattern: &[bool], source: &[u8]) -> Box<[bool]> {
    generic_get_pattern(false, pattern, source, |vec| {
        VP8Reader::new(Cursor::new(vec)).unwrap()
    })
}

#[inline(never)]
#[allow(dead_code)]
pub fn vp8_put_pattern_bypass(pattern: &[bool]) -> Vec<u8> {
    generic_put_pattern(true, pattern, || VP8Writer::new(Vec::new()).unwrap())
}

#[inline(never)]
#[allow(dead_code)]
pub fn vp8_get_pattern_bypass(pattern: &[bool], source: &[u8]) -> Box<[bool]> {
    generic_get_pattern(true, pattern, source, |vec| {
        VP8Reader::new(Cursor::new(vec)).unwrap()
    })
}

#[test]
fn vp8_test_pattern() {
    generic_test_pattern(vp8_get_pattern, vp8_put_pattern);
    generic_test_pattern(vp8_get_pattern_bypass, vp8_put_pattern_bypass);
}

#[inline(never)]
#[allow(dead_code)]
pub fn h265_put_pattern(pattern: &[bool]) -> Vec<u8> {
    generic_put_pattern(false, pattern, || H265Writer::new(Vec::new()))
}

#[inline(never)]
#[allow(dead_code)]
pub fn h265_put_pattern_bypass(pattern: &[bool]) -> Vec<u8> {
    generic_put_pattern(true, pattern, || H265Writer::new(Vec::new()))
}

#[inline(never)]
#[allow(dead_code)]
pub fn h265_get_pattern(pattern: &[bool], source: &[u8]) -> Box<[bool]> {
    generic_get_pattern(false, pattern, source, |vec| {
        H265Reader::new(Cursor::new(vec)).unwrap()
    })
}

#[inline(never)]
#[allow(dead_code)]
pub fn h265_get_pattern_bypass(pattern: &[bool], source: &[u8]) -> Box<[bool]> {
    generic_get_pattern(true, pattern, source, |vec| {
        H265Reader::new(Cursor::new(vec)).unwrap()
    })
}

#[test]
fn h264_test_pattern() {
    generic_test_pattern(h265_get_pattern, h265_put_pattern);
    generic_test_pattern(h265_get_pattern_bypass, h265_put_pattern_bypass);
}

#[inline(never)]
#[allow(dead_code)]
pub fn fpaq_put_pattern(pattern: &[bool]) -> Vec<u8> {
    generic_put_pattern(false, pattern, || Fpaq0Encoder::new(Vec::new()))
}

#[inline(never)]
#[allow(dead_code)]
pub fn fpaq_get_pattern(pattern: &[bool], source: &[u8]) -> Box<[bool]> {
    generic_get_pattern(false, pattern, source, |vec| {
        Fpaq0Decoder::new(Cursor::new(vec)).unwrap()
    })
}

#[test]
fn fpaq_test_pattern() {
    generic_test_pattern(fpaq_get_pattern, fpaq_put_pattern);
}

#[inline(never)]
#[allow(dead_code)]
pub fn fpaq_parallel_put_pattern(pattern: &[bool]) -> Vec<u8> {
    {
        let mut context = [
            VP8Context::default(),
            VP8Context::default(),
            VP8Context::default(),
            VP8Context::default(),
        ];

        let mut outputbuffer = Vec::new();
        let mut output = EncoderOutput::new(&mut outputbuffer);

        let mut writer = [
            Fpaq0EncoderParallel::new(&mut output, 0),
            Fpaq0EncoderParallel::new(&mut output, 1),
            Fpaq0EncoderParallel::new(&mut output, 2),
            Fpaq0EncoderParallel::new(&mut output, 3),
        ];

        for p in 0..pattern.len() / 4 {
            for i in 0..4 {
                writer[i]
                    .put(pattern[p * 4 + i], &mut context[i], &mut output)
                    .unwrap();
            }
        }

        for i in 0..4 {
            writer[i].finish(&mut output).unwrap();
        }

        outputbuffer
    }
}

#[cfg(feature = "simd")]
#[inline(never)]
#[allow(dead_code)]
pub fn fpaq_parallel_simd_get_pattern(pattern: &[bool], source: &[u8]) -> Box<[bool]> {
    let mut context = [
        VP8Context::default(),
        VP8Context::default(),
        VP8Context::default(),
        VP8Context::default(),
    ];

    let pattern = pattern;

    let mut output = vec![false; pattern.len()].into_boxed_slice();

    let mut input = Cursor::new(source);

    let mut reader = Fpaq0DecoderParallelSimd::new(&mut input).unwrap();

    assert!(output.len() == pattern.len());

    output.chunks_exact_mut(4).for_each(|chunk| {
        let r = reader.get(&mut context, &mut input).unwrap();
        chunk[0] = r & 1 != 0;
        chunk[1] = r & 2 != 0;
        chunk[2] = r & 4 != 0;
        chunk[3] = r & 8 != 0;
    });

    output
}

#[inline(never)]
#[allow(dead_code)]
pub fn fpaq_parallel_get_pattern(pattern: &[bool], source: &[u8]) -> Box<[bool]> {
    let mut context = [
        VP8Context::default(),
        VP8Context::default(),
        VP8Context::default(),
        VP8Context::default(),
    ];

    let pattern = pattern;

    let mut output = vec![false; pattern.len()].into_boxed_slice();

    let mut input = Cursor::new(source);

    let mut reader = [
        Fpaq0DecoderParallel::new(&mut input).unwrap(),
        Fpaq0DecoderParallel::new(&mut input).unwrap(),
        Fpaq0DecoderParallel::new(&mut input).unwrap(),
        Fpaq0DecoderParallel::new(&mut input).unwrap(),
    ];

    output.chunks_exact_mut(4).for_each(|chunk| {
        for i in 0..4 {
            chunk[i] = reader[i].get(&mut context[i], &mut input).unwrap();
        }
    });

    output
}

#[cfg(feature = "simd")]
#[test]
fn fpaq_parallel_simd_test_pattern() {
    generic_test_pattern(fpaq_parallel_simd_get_pattern, fpaq_parallel_put_pattern);
}

#[test]
fn fpaq_parallel_test_pattern() {
    generic_test_pattern(fpaq_parallel_get_pattern, fpaq_parallel_put_pattern);
}
