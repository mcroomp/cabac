use cabac::perf::{
    h265_get_pattern, h265_get_pattern_bypass, h265_put_pattern, h265_put_pattern_bypass,
    rans32_get_pattern, rans32_get_pattern_bypass, rans32_put_pattern, rans32_put_pattern_bypass,
    vp8_get_pattern, vp8_get_pattern_bypass, vp8_put_pattern, vp8_put_pattern_bypass,
};

use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let mut pattern = Vec::<bool>::new();
    rand::Rng::sample_iter(rand::thread_rng(), &rand::distributions::Standard)
        .take(65535)
        .for_each(|x| pattern.push(x));

    let rans_pattern = rans32_put_pattern(&pattern);
    let vp8_pattern = vp8_put_pattern(&pattern);
    let h265_pattern = h265_put_pattern(&pattern);

    let rans_pattern_bypass = rans32_put_pattern_bypass(&pattern);
    let vp8_pattern_bypass = vp8_put_pattern_bypass(&pattern);
    let h265_pattern_bypass = h265_put_pattern_bypass(&pattern);

    c.bench_function("VP8 read", |b| {
        b.iter(|| {
            vp8_get_pattern(&pattern, &vp8_pattern);
        })
    });

    c.bench_function("VP8 read bypass", |b| {
        b.iter(|| {
            vp8_get_pattern_bypass(&pattern, &vp8_pattern_bypass);
        })
    });

    c.bench_function("VP8 write", |b| {
        b.iter(|| {
            vp8_put_pattern(&pattern);
        })
    });

    c.bench_function("Rans32 read", |b| {
        b.iter(|| {
            rans32_get_pattern(&pattern, &rans_pattern);
        });
    });

    c.bench_function("Rans32 read bypass", |b| {
        b.iter(|| {
            rans32_get_pattern_bypass(&pattern, &rans_pattern_bypass);
        });
    });

    c.bench_function("Rans32 write", |b| {
        b.iter(|| {
            rans32_put_pattern(&pattern);
        })
    });

    c.bench_function("H265 read", |b| {
        b.iter(|| {
            h265_get_pattern(&pattern, &h265_pattern);
        });
    });

    c.bench_function("H265 read bypass", |b| {
        b.iter(|| {
            h265_get_pattern_bypass(&pattern, &h265_pattern_bypass);
        });
    });

    c.bench_function("H265 write", |b| {
        b.iter(|| {
            h265_put_pattern(&pattern);
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
