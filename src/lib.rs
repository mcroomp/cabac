pub mod cabac;
pub mod h265;
pub mod vp8;

#[cfg(test)]
use {
    cabac::{CabacReader, CabacWriter},
    std::io::Cursor,
};

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
fn test_permutation_h264(pattern: u64, num_bits: u8, bypass_index: u8) {
    use h265::{H265Context, H265Reader, H265Writer};

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

#[cfg(test)]
fn test_permutation_vp8(pattern: u64, num_bits: u8, bypass_index: u8) {
    use vp8::{VP8Context, VP8Reader, VP8Writer};

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

#[test]
fn test_vp8() {
    for k in 1..10 {
        for i in 0..(1 << (k - 1)) {
            test_permutation_vp8(i, k, k / 2);
        }
    }
}

#[test]
fn test_h264() {
    for k in 1..10 {
        for i in 0..(1 << (k - 1)) {
            test_permutation_h264(i, k, k / 2);
        }
    }
}
