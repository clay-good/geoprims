//! DEFLATE (RFC 1951). The compressor is deliberately simple and fully
//! deterministic: greedy LZ77 over hash chains with fixed parameters, emitted as
//! one final block with the fixed Huffman codes. Its bytes depend only on the
//! input, never on a host library, so the website and the MCP server produce
//! identical permalinks. The decompressor accepts any valid stream (stored,
//! fixed, and dynamic blocks) up to a size limit.

const WINDOW: usize = 32_768;
const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
const MAX_CHAIN: usize = 64;
const HASH_BITS: u32 = 15;

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

struct BitWriter {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl BitWriter {
    /// Writes `n` bits of `v`, least significant first.
    fn bits(&mut self, v: u32, n: u32) {
        self.acc |= v << self.n;
        self.n += n;
        while self.n >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }

    /// Writes a Huffman code, most significant bit first.
    fn code(&mut self, code: u32, len: u32) {
        let mut rev = 0;
        for i in 0..len {
            rev |= ((code >> i) & 1) << (len - 1 - i);
        }
        self.bits(rev, len);
    }

    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push(self.acc as u8);
        }
        self.out
    }
}

/// The fixed literal/length code for a symbol (RFC 1951 §3.2.6).
fn fixed_lit(w: &mut BitWriter, sym: u32) {
    match sym {
        0..=143 => w.code(0x30 + sym, 8),
        144..=255 => w.code(0x190 + sym - 144, 9),
        256..=279 => w.code(sym - 256, 7),
        _ => w.code(0xC0 + sym - 280, 8),
    }
}

fn hash(d: &[u8], i: usize) -> usize {
    let v = u32::from(d[i]) << 16 | u32::from(d[i + 1]) << 8 | u32::from(d[i + 2]);
    (v.wrapping_mul(0x9E37_79B1) >> (32 - HASH_BITS)) as usize
}

/// Compresses `data` to a raw DEFLATE stream.
pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut w = BitWriter {
        out: Vec::new(),
        acc: 0,
        n: 0,
    };
    w.bits(1, 1); // BFINAL
    w.bits(1, 2); // BTYPE = 01, fixed Huffman
    let mut head = vec![usize::MAX; 1 << HASH_BITS];
    let mut prev = vec![usize::MAX; data.len()];
    let mut i = 0;
    let insert = |head: &mut Vec<usize>, prev: &mut Vec<usize>, j: usize| {
        if j + MIN_MATCH <= data.len() {
            let h = hash(data, j);
            prev[j] = head[h];
            head[h] = j;
        }
    };
    while i < data.len() {
        let (mut best_len, mut best_dist) = (0, 0);
        if i + MIN_MATCH <= data.len() {
            let mut cand = head[hash(data, i)];
            let mut chain = 0;
            while cand != usize::MAX && i - cand <= WINDOW && chain < MAX_CHAIN {
                let max = (data.len() - i).min(MAX_MATCH);
                let len = (0..max)
                    .take_while(|&k| data[cand + k] == data[i + k])
                    .count();
                if len > best_len {
                    best_len = len;
                    best_dist = i - cand;
                    if len == max {
                        break;
                    }
                }
                cand = prev[cand];
                chain += 1;
            }
        }
        if best_len >= MIN_MATCH {
            let li = LEN_BASE
                .iter()
                .rposition(|&b| b as usize <= best_len)
                .expect("length >= 3");
            fixed_lit(&mut w, 257 + li as u32);
            w.bits(
                (best_len - LEN_BASE[li] as usize) as u32,
                u32::from(LEN_EXTRA[li]),
            );
            let di = DIST_BASE
                .iter()
                .rposition(|&b| b as usize <= best_dist)
                .expect("distance >= 1");
            w.code(di as u32, 5);
            w.bits(
                (best_dist - DIST_BASE[di] as usize) as u32,
                u32::from(DIST_EXTRA[di]),
            );
            for j in i..i + best_len {
                insert(&mut head, &mut prev, j);
            }
            i += best_len;
        } else {
            fixed_lit(&mut w, u32::from(data[i]));
            insert(&mut head, &mut prev, i);
            i += 1;
        }
    }
    fixed_lit(&mut w, 256);
    w.finish()
}

/// Why a stream could not be inflated.
#[derive(Debug, PartialEq, Eq)]
pub enum InflateError {
    Corrupt,
    TooLarge,
}

struct BitReader<'a> {
    d: &'a [u8],
    pos: usize,
    acc: u32,
    n: u32,
}

impl BitReader<'_> {
    fn bits(&mut self, need: u32) -> Result<u32, InflateError> {
        while self.n < need {
            let b = *self.d.get(self.pos).ok_or(InflateError::Corrupt)?;
            self.pos += 1;
            self.acc |= u32::from(b) << self.n;
            self.n += 8;
        }
        let v = self.acc & ((1u64 << need) - 1) as u32;
        self.acc >>= need;
        self.n -= need;
        Ok(v)
    }
}

/// A canonical Huffman decoding table: symbol counts per length, and symbols in order.
struct Huffman {
    count: [u16; 16],
    symbol: Vec<u16>,
}

impl Huffman {
    fn new(lengths: &[u8]) -> Result<Huffman, InflateError> {
        let mut count = [0u16; 16];
        for &l in lengths {
            count[l as usize] += 1;
        }
        count[0] = 0;
        let mut left: i32 = 1;
        for &c in &count[1..] {
            left = (left << 1) - i32::from(c);
            if left < 0 {
                return Err(InflateError::Corrupt);
            }
        }
        let mut offs = [0u16; 16];
        for l in 1..15 {
            offs[l + 1] = offs[l] + count[l];
        }
        let mut symbol = vec![0u16; lengths.len()];
        for (s, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbol[offs[l as usize] as usize] = s as u16;
                offs[l as usize] += 1;
            }
        }
        Ok(Huffman { count, symbol })
    }

    fn decode(&self, r: &mut BitReader) -> Result<u16, InflateError> {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..16 {
            code |= r.bits(1)? as i32;
            let count = i32::from(self.count[len]);
            if code - count < first {
                return self
                    .symbol
                    .get((index + code - first) as usize)
                    .copied()
                    .ok_or(InflateError::Corrupt);
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err(InflateError::Corrupt)
    }
}

/// Decompresses a raw DEFLATE stream, refusing output longer than `limit` bytes.
pub fn decompress(d: &[u8], limit: usize) -> Result<Vec<u8>, InflateError> {
    let mut r = BitReader {
        d,
        pos: 0,
        acc: 0,
        n: 0,
    };
    let mut out: Vec<u8> = Vec::new();
    loop {
        let last = r.bits(1)?;
        match r.bits(2)? {
            0 => {
                r.acc = 0;
                r.n = 0;
                let h = take(&mut r, 4)?;
                let (len, nlen) = (
                    u16::from_le_bytes([h[0], h[1]]),
                    u16::from_le_bytes([h[2], h[3]]),
                );
                if len != !nlen {
                    return Err(InflateError::Corrupt);
                }
                if out.len() + len as usize > limit {
                    return Err(InflateError::TooLarge);
                }
                let s = take(&mut r, len as usize)?;
                out.extend_from_slice(s);
            }
            1 => {
                let mut lengths = [0u8; 288];
                lengths[..144].fill(8);
                lengths[144..256].fill(9);
                lengths[256..280].fill(7);
                lengths[280..].fill(8);
                let lit = Huffman::new(&lengths)?;
                let dist = Huffman::new(&[5u8; 30])?;
                codes(&mut r, &mut out, &lit, &dist, limit)?;
            }
            2 => {
                let nlen = r.bits(5)? as usize + 257;
                let ndist = r.bits(5)? as usize + 1;
                let ncode = r.bits(4)? as usize + 4;
                if nlen > 286 || ndist > 30 {
                    return Err(InflateError::Corrupt);
                }
                const ORDER: [usize; 19] = [
                    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
                ];
                let mut cl = [0u8; 19];
                for &o in &ORDER[..ncode] {
                    cl[o] = r.bits(3)? as u8;
                }
                let clh = Huffman::new(&cl)?;
                let mut lengths = vec![0u8; nlen + ndist];
                let mut i = 0;
                while i < nlen + ndist {
                    let sym = clh.decode(&mut r)?;
                    let (val, rep) = match sym {
                        0..=15 => (sym as u8, 1),
                        16 => (
                            *lengths
                                .get(i.wrapping_sub(1))
                                .ok_or(InflateError::Corrupt)?,
                            3 + r.bits(2)? as usize,
                        ),
                        17 => (0, 3 + r.bits(3)? as usize),
                        _ => (0, 11 + r.bits(7)? as usize),
                    };
                    if i + rep > nlen + ndist {
                        return Err(InflateError::Corrupt);
                    }
                    lengths[i..i + rep].fill(val);
                    i += rep;
                }
                if lengths[256] == 0 {
                    return Err(InflateError::Corrupt);
                }
                let lit = Huffman::new(&lengths[..nlen])?;
                let dist = Huffman::new(&lengths[nlen..])?;
                codes(&mut r, &mut out, &lit, &dist, limit)?;
            }
            _ => return Err(InflateError::Corrupt),
        }
        if last == 1 {
            return Ok(out);
        }
    }
}

/// Takes `n` whole bytes after a stored-block header.
fn take<'a>(r: &mut BitReader<'a>, n: usize) -> Result<&'a [u8], InflateError> {
    let s = r.d.get(r.pos..r.pos + n).ok_or(InflateError::Corrupt)?;
    r.pos += n;
    Ok(s)
}

fn codes(
    r: &mut BitReader,
    out: &mut Vec<u8>,
    lit: &Huffman,
    dist: &Huffman,
    limit: usize,
) -> Result<(), InflateError> {
    loop {
        let sym = lit.decode(r)? as usize;
        if sym < 256 {
            if out.len() >= limit {
                return Err(InflateError::TooLarge);
            }
            out.push(sym as u8);
        } else if sym == 256 {
            return Ok(());
        } else {
            let li = sym - 257;
            if li >= 29 {
                return Err(InflateError::Corrupt);
            }
            let len = LEN_BASE[li] as usize + r.bits(u32::from(LEN_EXTRA[li]))? as usize;
            let di = dist.decode(r)? as usize;
            if di >= 30 {
                return Err(InflateError::Corrupt);
            }
            let d = DIST_BASE[di] as usize + r.bits(u32::from(DIST_EXTRA[di]))? as usize;
            if d > out.len() {
                return Err(InflateError::Corrupt);
            }
            if out.len() + len > limit {
                return Err(InflateError::TooLarge);
            }
            let start = out.len() - d;
            for k in 0..len {
                out.push(out[start + k]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let mut s: u64 = 42;
        for n in [0usize, 1, 2, 3, 10, 100, 1000, 70_000] {
            let data: Vec<u8> = (0..n)
                .map(|i| {
                    s ^= s << 13;
                    s ^= s >> 7;
                    s ^= s << 17;
                    if i % 3 == 0 {
                        b'a' + (s % 4) as u8
                    } else {
                        (s >> 20) as u8
                    }
                })
                .collect();
            let c = compress(&data);
            assert_eq!(decompress(&c, usize::MAX).unwrap(), data, "n = {n}");
        }
        let rep = b"abcabcabcabcabcabcabcabcabcabcabc".repeat(50);
        let c = compress(&rep);
        assert!(c.len() < rep.len() / 10);
        assert_eq!(decompress(&c, usize::MAX).unwrap(), rep);
    }

    #[test]
    fn limit_and_corruption() {
        let c = compress(&[b'x'; 10_000]);
        assert_eq!(decompress(&c, 100), Err(InflateError::TooLarge));
        assert_eq!(decompress(&[0xff, 0xff], 100), Err(InflateError::Corrupt));
        assert_eq!(decompress(&[], 100), Err(InflateError::Corrupt));
    }
}
