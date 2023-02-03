use std::io::Cursor;

use cabac::traits::{CabacReader, CabacWriter};

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
    use cabac::h265::{H265Context, H265Reader, H265Writer};

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
    use cabac::vp8::{VP8Context, VP8Reader, VP8Writer};

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

enum Seq {
    Normal(bool),
    Bypass(bool),
}

fn test_seq_vp8(seq: &[Seq]) {
    use cabac::vp8::{VP8Context, VP8Reader, VP8Writer};

    let mut output = Vec::new();
    {
        let mut context = VP8Context::default();
        let mut writer = VP8Writer::new(&mut output).unwrap();

        for s in seq {
            match s {
                Seq::Normal(b) => writer.put(*b, &mut context).unwrap(),
                Seq::Bypass(b) => writer.put_bypass(*b).unwrap(),
            }
        }

        writer.finish().unwrap();
    }

    // now try reading it
    {
        let mut context = VP8Context::default();
        let mut reader = VP8Reader::new(Cursor::new(&output)).unwrap();

        for s in seq {
            match s {
                Seq::Normal(b) => {
                    assert_eq!(*b, reader.get(&mut context).unwrap())
                }
                Seq::Bypass(b) => {
                    assert_eq!(*b, reader.get_bypass().unwrap())
                }
            }
        }
    }
}

fn test_seq_h265(seq: &[Seq]) {
    use cabac::h265::{H265Context, H265Reader, H265Writer};

    let mut output = Vec::new();
    {
        let mut context = H265Context::default();
        let mut writer = H265Writer::new(&mut output);

        for s in seq {
            match s {
                Seq::Normal(b) => writer.put(*b, &mut context).unwrap(),
                Seq::Bypass(b) => writer.put_bypass(*b).unwrap(),
            }
        }

        writer.finish().unwrap();
    }

    // now try reading it
    {
        let mut context = H265Context::default();
        let mut reader = H265Reader::new(Cursor::new(&output)).unwrap();

        for s in seq {
            match s {
                Seq::Normal(b) => {
                    assert_eq!(*b, reader.get(&mut context).unwrap())
                }
                Seq::Bypass(b) => {
                    assert_eq!(*b, reader.get_bypass().unwrap())
                }
            }
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
    let mut seed: u32 = 27;

    let mut seq = Vec::new();

    for i in 0..10000 {
        seed = seed.wrapping_mul(10000019) + 7;

        seq.push(match seed % 4 {
            0 => Seq::Normal(true),
            1 => Seq::Normal(false),
            2 => Seq::Bypass(false),
            _ => Seq::Bypass(true),
        });
    }

    test_seq_h265(&seq);
    test_seq_vp8(&seq);
}
