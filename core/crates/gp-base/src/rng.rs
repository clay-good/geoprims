//! The one seeded generator the sampling tools may use (add-sampling-and-grids
//! S1): xoshiro256** (Blackman & Vigna 2021), its state filled by splitmix64
//! from a 64-bit seed as the authors suggest, and doubles drawn as
//! (x >> 11) × 2⁻⁵³. Pure integer arithmetic: no host randomness, so the same
//! seed gives the same stream in every build.

/// splitmix64 (Vigna, public domain), used only to fill the xoshiro state.
fn splitmix64(x: &mut u64) -> u64 {
    *x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *x;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// xoshiro256**: 256 bits of state, period 2²⁵⁶ − 1.
#[derive(Clone, Debug)]
pub struct Xoshiro256 {
    s: [u64; 4],
}

impl Xoshiro256 {
    /// Seeds the state with four splitmix64 outputs from `seed`.
    pub fn new(seed: u64) -> Self {
        let mut x = seed;
        Xoshiro256 {
            s: [
                splitmix64(&mut x),
                splitmix64(&mut x),
                splitmix64(&mut x),
                splitmix64(&mut x),
            ],
        }
    }

    /// The next 64-bit output.
    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.s;
        let result = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }

    /// A double in [0, 1): the top 53 bits of the next output, times 2⁻⁵³.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::Xoshiro256;

    // The first 10 outputs, from a C transcription of the authors' public-domain
    // reference code (xoshiro256starstar.c and splitmix64.c), compiled with clang.
    const SEED_0: [u64; 10] = [
        11091344671253066420,
        13793997310169335082,
        1900383378846508768,
        7684712102626143532,
        13521403990117723737,
        18442103541295991498,
        7788427924976520344,
        9881088229871127103,
        15781505947799885617,
        16949938600482740797,
    ];
    const SEED_42: [u64; 10] = [
        1546998764402558742,
        6990951692964543102,
        12544586762248559009,
        17057574109182124193,
        18295552978065317476,
        14199186830065750584,
        13267978908934200754,
        15679888225317814407,
        14044878350692344958,
        10760895422300929085,
    ];

    #[test]
    fn reference_streams() {
        for (seed, want) in [(0, SEED_0), (42, SEED_42)] {
            let mut g = Xoshiro256::new(seed);
            let got: Vec<u64> = (0..10).map(|_| g.next_u64()).collect();
            assert_eq!(got, want, "seed {seed}");
        }
        let mut g = Xoshiro256::new(u64::MAX);
        assert_eq!(g.next_u64(), 10328197420357168392);
    }

    #[test]
    fn doubles_take_the_top_53_bits() {
        let mut g = Xoshiro256::new(42);
        assert_eq!(g.next_f64(), 0.08386297105988216);
        assert_eq!(g.next_f64(), 0.3789802506626686);
        let mut g = Xoshiro256::new(0);
        for _ in 0..10_000 {
            let v = g.next_f64();
            assert!((0.0..1.0).contains(&v));
        }
    }
}
