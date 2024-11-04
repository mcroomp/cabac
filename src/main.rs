use cabac::perf::{
    fpaq_parallel_get_pattern, fpaq_parallel_put_pattern, fpaq_parallel_simd_get_pattern,
    rans32_get_pattern, rans32_get_pattern_bypass, rans32_put_pattern, rans32_put_pattern_bypass,
    vp8_get_pattern, vp8_put_pattern,
};

/// Generates the next pseudo-random number.
/// Definitely non-cryptographic, just used for generating random test values.
const fn next_rand_u64(state: u64) -> u64 {
    // Constants for the LCG
    const A: u64 = 6364136223846793005;
    const C: u64 = 1442695040888963407;

    // Update the state and calculate the next number (rotate to avoid lack of
    // randomness in low bits)
    state.wrapping_mul(A).wrapping_add(C).rotate_left(31)
}

const RNG_SEED: u64 = 0x123456789abcdef0;

const fn gen_pattern() -> [bool; 10240] {
    let mut pattern = [false; 10240];
    let mut rng = RNG_SEED;

    let mut i = 0;
    while i < 1000 {
        pattern[i] = false;
        i += 1;
    }
    while i < 2000 {
        pattern[i] = true;
        i += 1;
    }
    while i < 3000 {
        rng = next_rand_u64(rng);
        pattern[i] = rng % 2 == 0;
        i += 1;
    }
    while i < 4000 {
        rng = next_rand_u64(rng);
        pattern[i] = rng % 10 == 0;
        i += 1;
    }
    while i < 5000 {
        rng = next_rand_u64(rng);
        pattern[i] = rng % 30 == 0;
        i += 1;
    }
    while i < 6000 {
        rng = next_rand_u64(rng);
        pattern[i] = rng % 30 != 0;
        i += 1;
    }
    while i < 7000 {
        rng = next_rand_u64(rng);
        pattern[i] = rng % 10 != 0;
        i += 1;
    }
    while i < 8000 {
        rng = next_rand_u64(rng);
        pattern[i] = rng % 5 != 0;
        i += 1;
    }
    while i < 9000 {
        rng = next_rand_u64(rng);
        pattern[i] = rng % 6 != 0;
        i += 1;
    }
    while i < 9500 {
        rng = next_rand_u64(rng);
        pattern[i] = rng % 9 == 0;
        i += 1;
    }
    while i < 10240 {
        rng = next_rand_u64(rng);
        pattern[i] = rng % 2 == 0;
        i += 1;
    }

    pattern
}

static BOOL_PATTERN: [bool; 10240] = gen_pattern();

fn main() {
    for i in 0..100000 {
        /*let v = vp8_put_pattern(&BOOL_PATTERN);
        let o = vp8_get_pattern(&BOOL_PATTERN, &v);
        if i == 0 {
            assert!(o[..] == BOOL_PATTERN);
        }

        let v = rans32_put_pattern(&BOOL_PATTERN);
        let o = rans32_get_pattern(&BOOL_PATTERN, &v);
        if i == 0 {
            assert!(o[..] == BOOL_PATTERN);
        }

        let v = rans32_put_pattern_bypass(&BOOL_PATTERN);
        let o = rans32_get_pattern_bypass(&BOOL_PATTERN, &v);
        if i == 0 {
            assert!(o[..] == BOOL_PATTERN);
        }*/

        let v = fpaq_parallel_put_pattern(&BOOL_PATTERN);
        let o = fpaq_parallel_get_pattern(&BOOL_PATTERN, &v);
        if i == 0 {
            assert!(o[..] == BOOL_PATTERN);
        }
        let o = fpaq_parallel_simd_get_pattern(&BOOL_PATTERN, &v);
        if i == 0 {
            assert!(o[..] == BOOL_PATTERN);
        }
    }
}
