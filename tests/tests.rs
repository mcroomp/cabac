use std::io::Cursor;

use cabac::h265::{H265Context, H265Reader, H265Writer};
use cabac::traits::{CabacReader, CabacWriter};
use cabac::vp8::{VP8Context, VP8Reader, VP8Writer};

fn set_bits<CONTEXT, CW: CabacWriter<CONTEXT>>(
    writer: &mut CW,
    context: &mut CONTEXT,
    pattern: u64,
    num_bits: u8,
    bypass_index: u8,
) {
    for i in 0..num_bits {
        let v = (pattern & (1 << i)) != 0;
        if i == bypass_index {
            writer.put_bypass(v).unwrap();
        } else {
            writer.put(v, context).unwrap();
        }
    }

    writer.finish().unwrap();
}

fn test_bits<CONTEXT, CR: CabacReader<CONTEXT>>(
    reader: &mut CR,
    context: &mut CONTEXT,
    pattern: u64,
    num_bits: u8,
    bypass_index: u8,
) {
    for i in 0..num_bits {
        let bit;
        if i == bypass_index {
            bit = reader.get_bypass().unwrap();
        } else {
            bit = reader.get(context).unwrap();
        }

        assert!(
            ((pattern & (1 << i)) != 0) == bit,
            "Pattern {0:b}-{1} iter {2} fail was {3}",
            pattern,
            num_bits,
            i,
            bit
        );
    }
}

fn test_permutation_h264(pattern: u64, num_bits: u8, bypass_index: u8) {
    let mut output = Vec::new();
    {
        let mut context = H265Context::default();
        let mut writer = H265Writer::new(&mut output);
        set_bits(&mut writer, &mut context, pattern, num_bits, bypass_index);
    }

    // now try reading it
    {
        let mut context = H265Context::default();
        let mut reader = H265Reader::new(Cursor::new(&output)).unwrap();

        test_bits(&mut reader, &mut context, pattern, num_bits, bypass_index);
    }
}

fn test_permutation_vp8(pattern: u64, num_bits: u8, bypass_index: u8) {
    let mut output = Vec::new();
    {
        let mut context = VP8Context::default();
        let mut writer = VP8Writer::new(&mut output).unwrap();
        set_bits(&mut writer, &mut context, pattern, num_bits, bypass_index);
    }

    // now try reading it
    {
        let mut context = VP8Context::default();
        let mut reader = VP8Reader::new(Cursor::new(&output)).unwrap();

        test_bits(&mut reader, &mut context, pattern, num_bits, bypass_index);
    }
}

#[derive(Clone, Copy)]
enum Seq {
    Normal(bool, usize),
    Bypass(bool),
}

fn test_seq_vp8(seq: &[Seq]) {
    let mut output = Vec::new();
    {
        let mut context = Vec::new();
        for _i in 0..16 {
            context.push(VP8Context::default());
        }
        let mut writer = VP8Writer::new(&mut output).unwrap();

        for &s in seq {
            match s {
                Seq::Normal(b, c) => writer.put(b, &mut context[c]).unwrap(),
                Seq::Bypass(b) => writer.put_bypass(b).unwrap(),
            }
        }

        writer.finish().unwrap();
    }

    // now try reading it
    {
        let mut context = Vec::new();
        for _ in 0..16 {
            context.push(VP8Context::default());
        }

        let mut reader = VP8Reader::new(Cursor::new(&output)).unwrap();

        for &s in seq {
            match s {
                Seq::Normal(b, c) => {
                    assert_eq!(b, reader.get(&mut context[c]).unwrap())
                }
                Seq::Bypass(b) => {
                    assert_eq!(b, reader.get_bypass().unwrap())
                }
            }
        }
    }
}

fn test_seq_h265(seq: &[Seq]) {
    let mut output = Vec::new();
    {
        let mut context = Vec::new();
        for _ in 0..16 {
            context.push(H265Context::default());
        }

        let mut writer = H265Writer::new(&mut output);

        for &s in seq {
            match s {
                Seq::Normal(b, c) => writer.put(b, &mut context[c]).unwrap(),
                Seq::Bypass(b) => writer.put_bypass(b).unwrap(),
            }
        }

        writer.finish().unwrap();
    }

    // now try reading it
    {
        let mut context = Vec::new();
        for _ in 0..16 {
            context.push(H265Context::default());
        }

        let mut reader = H265Reader::new(Cursor::new(&output)).unwrap();

        for &s in seq {
            match s {
                Seq::Normal(b, c) => {
                    assert_eq!(b, reader.get(&mut context[c]).unwrap())
                }
                Seq::Bypass(b) => {
                    assert_eq!(b, reader.get_bypass().unwrap())
                }
            }
        }
    }
}

#[test]
fn bypass_vp8() {
    let mut output = Vec::new();
    {
        let mut writer = VP8Writer::new(&mut output).unwrap();
        for i in 0..1024 {
            writer.put_bypass((i & 1) != 0).unwrap();
        }

        writer.finish().unwrap();
    }

    {
        let mut reader = VP8Reader::new(Cursor::new(&output)).unwrap();
        for i in 0..1024 {
            assert_eq!(reader.get_bypass().unwrap(), (i & 1) != 0);
        }
    }
}

#[test]
fn bypass_h265() {
    let mut output = Vec::new();
    {
        let mut writer = H265Writer::new(&mut output);
        for i in 0..1024 {
            writer.put_bypass((i & 1) != 0).unwrap();
        }

        writer.finish().unwrap();
    }

    {
        let mut reader = H265Reader::new(Cursor::new(&output)).unwrap();
        for i in 0..1024 {
            assert_eq!(reader.get_bypass().unwrap(), (i & 1) != 0);
        }
    }
}

#[test]
fn test_basic_permutations_vp8() {
    for k in 1..10 {
        for i in 0..(1 << (k - 1)) {
            test_permutation_vp8(i, k, k / 2);
        }
    }
}

#[test]
fn test_basic_permutations_h264() {
    for k in 1..10 {
        for i in 0..(1 << (k - 1)) {
            test_permutation_h264(i, k, k / 2);
        }
    }
}

#[test]
fn test_random_sequences() {
    use rand::Rng;

    let mut rng = rand::thread_rng();

    let probs: [f64; 16] = [
        0.001, 0.01, 0.1, 0.11, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.9, 0.91, 0.99, 0.999, 0.9999, 1.0,
    ];

    for _ in 1..1000 {
        let mut seq = Vec::new();

        for _ in 0..1000 {
            let ctx = rng.gen_range(0..16);

            seq.push(match rng.gen_range(0..4) {
                0 | 1 => Seq::Normal(rng.gen_bool(probs[ctx]), ctx),
                2 => Seq::Bypass(false),
                _ => Seq::Bypass(true),
            });
        }

        test_seq_h265(&seq);
        test_seq_vp8(&seq);
    }
}

#[test]
fn test_all_0() {
    let all_0 = vec![Seq::Normal(false, 0); 10000];

    test_seq_h265(&all_0);
    test_seq_vp8(&all_0);
}

#[test]
fn test_all_1() {
    let all_1 = vec![Seq::Normal(true, 0); 10000];

    test_seq_h265(&all_1);
    test_seq_vp8(&all_1);
}

#[test]
fn test_alt() {
    let mut seq = Vec::new();
    for i in 0..10000 {
        seq.push(Seq::Normal(i % 2 == 0, 0));
    }
    test_seq_h265(&seq);
    test_seq_vp8(&seq);
}
