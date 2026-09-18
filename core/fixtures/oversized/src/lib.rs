const N: usize = 512 * 1024;

static NOISE: [u8; N] = {
    let mut out = [0u8; N];
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut i = 0;
    while i < N {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        out[i] = (s >> 32) as u8;
        i += 1;
    }
    out
};

#[unsafe(no_mangle)]
pub extern "C" fn noise(i: usize) -> u8 {
    NOISE[i % N]
}
