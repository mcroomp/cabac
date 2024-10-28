use std::io::Cursor;

use cabac::h265::{H265Reader, H265Writer};
use cabac::rans32::{RansReader32, RansWriter32};
use cabac::rans64::{RansReader64, RansWriter64};
use cabac::traits::{CabacReader, CabacWriter, GetInnerBuffer};

use cabac::vp8::{VP8Reader, VP8Writer};
use criterion::{criterion_group, criterion_main, Bencher, Criterion};

const fn gen_pattern() -> [bool; 1024] {
    let mut pattern = [false; 1024];
    let mut i = 0;
    while i < 100 {
        pattern[i] = false;
        i += 1;
    }
    while i < 200 {
        pattern[i] = true;
        i += 1;
    }
    while i < 300 {
        pattern[i] = i % 2 == 0;
        i += 1;
    }
    while i < 400 {
        pattern[i] = i % 10 == 0;
        i += 1;
    }
    while i < 400 {
        pattern[i] = i % 30 == 0;
        i += 1;
    }
    while i < 500 {
        pattern[i] = i % 30 != 0;
        i += 1;
    }
    while i < 600 {
        pattern[i] = i % 10 != 0;
        i += 1;
    }
    while i < 700 {
        pattern[i] = i % 5 != 0;
        i += 1;
    }
    while i < 800 {
        pattern[i] = i % 6 != 0;
        i += 1;
    }
    while i < 900 {
        pattern[i] = i % 9 == 0;
        i += 1;
    }
    while i < 1024 {
        pattern[i] = i % 2 == 0;
        i += 1;
    }

    pattern
}

const BOOL_PATTERN: [bool; 1024] = gen_pattern();

fn pattern(i: i32) -> bool {
    BOOL_PATTERN[(i & 1023) as usize]
}

fn alternating_get_init<CONTEXT: Default, CW: CabacWriter<CONTEXT>>(writer: &mut CW) {
    let mut context = CONTEXT::default();
    for i in 0..1024 {
        writer.put(pattern(i), &mut context).unwrap();
    }
}

fn alternating_get_run<CONTEXT: Default, CR: CabacReader<CONTEXT>>(reader: &mut CR) {
    let mut context = CONTEXT::default();
    for i in 0..1024 {
        assert_eq!(pattern(i), reader.get(&mut context).unwrap());
    }
}

fn bypass_init<CONTEXT: Default, CW: CabacWriter<CONTEXT>>(writer: &mut CW) {
    for i in 0..1024 {
        writer.put_bypass(pattern(i)).unwrap();
    }
}

fn bypass_run<CONTEXT: Default, CR: CabacReader<CONTEXT>>(reader: &mut CR) {
    for i in 0..1024 {
        assert_eq!(pattern(i), reader.get_bypass().unwrap())
    }
}

fn test_batch<
    C: Default,
    R: CabacReader<C>,
    W: CabacWriter<C> + GetInnerBuffer,
    FW: Fn() -> W,
    FR: Fn(Vec<u8>) -> R,
>(
    b: &mut Bencher,
    fr: FR,
    fw: FW,
    init_fn: fn(&mut W),
    run_fn: fn(&mut R),
) {
    b.iter_batched(
        || {
            let mut w = fw();

            init_fn(&mut w);

            w.finish().unwrap();

            //state.init(&mut output, |cw| init_fn(cw));
            w.inner_buffer().to_vec()
        },
        |s| {
            let mut r = fr(s);
            run_fn(&mut r);
        },
        criterion::BatchSize::LargeInput,
    );
}

/*



impl RState
{
    fn get_init<CONTEXT : Default, CW : CabacWriter<CONTEXT>>(writer: &mut CW)
    {
        let mut context = CONTEXT::default();
        for i in 0..1024
        {
            writer.put((i & 1) != 0, &mut context).unwrap();
        }
    }

    fn get_test<CONTEXT : Default, CR : CabacReader<CONTEXT>>(reader: &mut CR)
    {
        let mut context = CONTEXT::default();
        for _i in 0..1024
        {
           let _ = reader.get(&mut context);
        }
    }

    fn get_bypass_init<CONTEXT : Default, CW : CabacWriter<CONTEXT>>(writer: &mut CW)
    {
        let mut context = CONTEXT::default();
        for i in 0..1024
        {
            writer.put((i & 1) != 0, &mut context).unwrap();
        }
    }

    fn get_bypass_test<CONTEXT : Default, CR : CabacReader<CONTEXT>>(reader: &mut CR)
    {
        let mut context = CONTEXT::default();
        for _i in 0..1024
        {
           let _ = reader.get(&mut context);
        }
    }

    fn init_h265()-> Self
    {
        let mut output = Vec::new();
        let mut writer = H265Writer::new(&mut output);

        Self::get_init(&mut writer);

        RState {output : output}
    }

    fn run_h265(&self)
    {
        let mut reader = H265Reader::new(Cursor::new(&self.output)).unwrap();

        Self::get_test(&mut reader);
    }


    fn init_h265_bypass()-> Self
    {
        let mut output = Vec::new();
        let mut writer = H265Writer::new(&mut output);

        Self::get_bypass_init(&mut writer);

        RState {output : output}
    }

    fn run_h265_bypass(&self)
    {
        let mut reader = H265Reader::new(Cursor::new(&self.output)).unwrap();

        Self::get_bypass_test(&mut reader);
    }


    fn init_vp8()-> Self
    {
        let mut output = Vec::new();
        let mut context = VP8Context::default();
        let mut writer = VP8Writer::new(&mut output).unwrap();

        Self::get_init(&mut writer);

        RState {output : output}
    }

    fn run_vp8(&self)
    {
        let mut reader = VP8Reader::new(Cursor::new(&self.output)).unwrap();

        Self::get_test(&mut reader);
    }

}
*/

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("VP8 read", |b| {
        test_batch(
            b,
            |v| VP8Reader::new(Cursor::new(v)).unwrap(),
            || VP8Writer::new(Vec::new()).unwrap(),
            |r| alternating_get_init(r),
            |r| alternating_get_run(r),
        );
    });

    c.bench_function("Rans64 read", |b| {
        test_batch(
            b,
            |v| RansReader64::new(Cursor::new(v)).unwrap(),
            || RansWriter64::new(Vec::new()),
            |r| alternating_get_init(r),
            |r| alternating_get_run(r),
        );
    });

    c.bench_function("Rans32 read", |b| {
        test_batch(
            b,
            |v| RansReader32::new(Cursor::new(v)).unwrap(),
            || RansWriter32::new(Vec::new()),
            |r| alternating_get_init(r),
            |r| alternating_get_run(r),
        );
    });

    c.bench_function("H265 read", |b| {
        test_batch(
            b,
            |v| H265Reader::new(Cursor::new(v)).unwrap(),
            || H265Writer::new(Vec::new()),
            |r| alternating_get_init(r),
            |r| alternating_get_run(r),
        );
    });

    c.bench_function("VP8 bypass", |b| {
        test_batch(
            b,
            |v| VP8Reader::new(Cursor::new(v)).unwrap(),
            || VP8Writer::new(Vec::new()).unwrap(),
            |r| bypass_init(r),
            |r| bypass_run(r),
        );
    });

    c.bench_function("H265 bypass", |b| {
        test_batch(
            b,
            |v| H265Reader::new(Cursor::new(v)).unwrap(),
            || H265Writer::new(Vec::new()),
            |r| bypass_init(r),
            |r| bypass_run(r),
        );
    });

    c.bench_function("Rans bypass", |b| {
        test_batch(
            b,
            |v| RansReader64::new(Cursor::new(v)).unwrap(),
            || RansWriter64::new(Vec::new()),
            |r| bypass_init(r),
            |r| bypass_run(r),
        );
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
