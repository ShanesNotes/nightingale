#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Digest {
    bytes: [u8; 32],
}
impl Digest {
    /// SHA-256 over canonical bytes.
    pub fn of_canonical(bytes: &[u8]) -> Self {
        Self { bytes: sha256(bytes) }
    }
    pub fn for_record(bytes: &[u8], hash: RecordHash) -> Self {
        let bytes = match hash {
            RecordHash::Sha256 => sha256(bytes),
            RecordHash::Blake2b => blake2b(bytes),
        };
        Self { bytes }
    }
    /// A digest is a value, never a capability.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordHash {
    Sha256,
    /// RFC 7693, unkeyed, with a 32-byte digest.
    Blake2b,
}

/// FIPS 180-4, section 4.2.2.
const ROUND: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];
/// FIPS 180-4, section 5.3.3.
const INITIAL: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

fn sha256(message: &[u8]) -> [u8; 32] {
    let mut state = INITIAL;
    let (blocks, rest) = message.as_chunks::<64>();
    for block in blocks {
        compress(&mut state, block);
    }
    let mut tail = [0u8; 128];
    tail[..rest.len()].copy_from_slice(rest);
    tail[rest.len()] = 0x80;
    let end = if rest.len() < 56 { 64 } else { 128 };
    let bits = (message.len() as u64).wrapping_mul(8);
    tail[end - 8..end].copy_from_slice(&bits.to_be_bytes());
    for block in tail[..end].as_chunks::<64>().0 {
        compress(&mut state, block);
    }
    let mut out = [0u8; 32];
    for (bytes, word) in out.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        *bytes = word.to_be_bytes();
    }
    out
}

fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut schedule = [0u32; 64];
    for (word, bytes) in schedule.iter_mut().zip(block.as_chunks::<4>().0) {
        *word = u32::from_be_bytes(*bytes);
    }
    for t in 16..64 {
        let (early, late) = (schedule[t - 15], schedule[t - 2]);
        let sigma0 = early.rotate_right(7) ^ early.rotate_right(18) ^ (early >> 3);
        let sigma1 = late.rotate_right(17) ^ late.rotate_right(19) ^ (late >> 10);
        schedule[t] = schedule[t - 16]
            .wrapping_add(sigma0)
            .wrapping_add(schedule[t - 7])
            .wrapping_add(sigma1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for (constant, word) in ROUND.iter().zip(schedule) {
        let big_sigma1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let choose = (e & f) ^ (!e & g);
        let first = h
            .wrapping_add(big_sigma1)
            .wrapping_add(choose)
            .wrapping_add(*constant)
            .wrapping_add(word);
        let big_sigma0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let second = big_sigma0.wrapping_add(majority);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(first);
        d = c;
        c = b;
        b = a;
        a = first.wrapping_add(second);
    }
    for (word, add) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *word = word.wrapping_add(add);
    }
}

/// RFC 7693, section 2.6.
const BLAKE2B_INITIAL: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];
/// RFC 7693, section 2.7.
const SIGMA: [[usize; 16]; 12] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
];

fn blake2b(message: &[u8]) -> [u8; 32] {
    let mut state = BLAKE2B_INITIAL;
    // RFC 7693, section 2.5: depth 1, fanout 1, no key, a 32-byte digest.
    state[0] ^= 0x0101_0020;
    let before_last = message.len().saturating_sub(1) / 128;
    let (blocks, _) = message.as_chunks::<128>();
    for (at, block) in blocks[..before_last].iter().enumerate() {
        blake2b_block(&mut state, block, 128 * (at as u128 + 1), false);
    }
    let tail = &message[128 * before_last..];
    let mut last = [0u8; 128];
    last[..tail.len()].copy_from_slice(tail);
    blake2b_block(&mut state, &last, message.len() as u128, true);
    let mut out = [0u8; 32];
    for (bytes, word) in out.as_chunks_mut::<8>().0.iter_mut().zip(state) {
        *bytes = word.to_le_bytes();
    }
    out
}

fn blake2b_block(state: &mut [u64; 8], block: &[u8; 128], counted: u128, last: bool) {
    let mut words = [0u64; 16];
    for (word, bytes) in words.iter_mut().zip(block.as_chunks::<8>().0) {
        *word = u64::from_le_bytes(*bytes);
    }
    let mut work = [0u64; 16];
    work[..8].copy_from_slice(state);
    work[8..].copy_from_slice(&BLAKE2B_INITIAL);
    work[12] ^= counted as u64;
    work[13] ^= (counted >> 64) as u64;
    if last {
        work[14] = !work[14];
    }
    round::<0>(&mut work, &words);
    round::<1>(&mut work, &words);
    round::<2>(&mut work, &words);
    round::<3>(&mut work, &words);
    round::<4>(&mut work, &words);
    round::<5>(&mut work, &words);
    round::<6>(&mut work, &words);
    round::<7>(&mut work, &words);
    round::<8>(&mut work, &words);
    round::<9>(&mut work, &words);
    round::<10>(&mut work, &words);
    round::<11>(&mut work, &words);
    for (word, (low, high)) in state.iter_mut().zip(work[..8].iter().zip(&work[8..])) {
        *word ^= low ^ high;
    }
}

/// Inlined by force, with `mix`: left to itself the compiler keeps a block's 96 mixes out of line, slower than
/// SHA-256.
#[inline(always)]
fn round<const R: usize>(work: &mut [u64; 16], words: &[u64; 16]) {
    let s = &SIGMA[R];
    mix(work, [0, 4, 8, 12], words[s[0]], words[s[1]]);
    mix(work, [1, 5, 9, 13], words[s[2]], words[s[3]]);
    mix(work, [2, 6, 10, 14], words[s[4]], words[s[5]]);
    mix(work, [3, 7, 11, 15], words[s[6]], words[s[7]]);
    mix(work, [0, 5, 10, 15], words[s[8]], words[s[9]]);
    mix(work, [1, 6, 11, 12], words[s[10]], words[s[11]]);
    mix(work, [2, 7, 8, 13], words[s[12]], words[s[13]]);
    mix(work, [3, 4, 9, 14], words[s[14]], words[s[15]]);
}

/// RFC 7693, section 3.1.
#[inline(always)]
fn mix(work: &mut [u64; 16], [a, b, c, d]: [usize; 4], x: u64, y: u64) {
    work[a] = work[a].wrapping_add(work[b]).wrapping_add(x);
    work[d] = (work[d] ^ work[a]).rotate_right(32);
    work[c] = work[c].wrapping_add(work[d]);
    work[b] = (work[b] ^ work[c]).rotate_right(24);
    work[a] = work[a].wrapping_add(work[b]).wrapping_add(y);
    work[d] = (work[d] ^ work[a]).rotate_right(16);
    work[c] = work[c].wrapping_add(work[d]);
    work[b] = (work[b] ^ work[c]).rotate_right(63);
}

pub(crate) struct Canonical {
    bytes: Vec<u8>,
}
impl Canonical {
    pub(crate) fn new(tag: &str) -> Self {
        let mut canonical = Self { bytes: Vec::new() };
        canonical.text(tag);
        canonical
    }
    pub(crate) fn number(&mut self, number: u64) {
        self.bytes.extend_from_slice(&number.to_be_bytes());
    }
    pub(crate) fn length(&mut self, length: usize) {
        self.number(length as u64);
    }
    pub(crate) fn text(&mut self, text: &str) {
        self.number(text.len() as u64);
        self.bytes.extend_from_slice(text.as_bytes());
    }
    pub(crate) fn raw(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }
    pub(crate) fn digest(&self) -> Digest {
        Digest::of_canonical(&self.bytes)
    }
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

pub(crate) trait Seal {
    fn seal(&self, out: &mut Canonical);
}
impl<T: Seal> Seal for [T] {
    fn seal(&self, out: &mut Canonical) {
        out.length(self.len());
        for item in self {
            item.seal(out);
        }
    }
}
impl<T: Seal> Seal for Option<T> {
    fn seal(&self, out: &mut Canonical) {
        match self {
            None => out.number(0),
            Some(value) => {
                out.number(1);
                value.seal(out);
            }
        }
    }
}
impl Seal for str {
    fn seal(&self, out: &mut Canonical) {
        out.text(self);
    }
}
impl<A: Seal, B: Seal> Seal for (A, B) {
    fn seal(&self, out: &mut Canonical) {
        self.0.seal(out);
        self.1.seal(out);
    }
}
impl Seal for i64 {
    fn seal(&self, out: &mut Canonical) {
        out.number(*self as u64);
    }
}
impl Seal for u64 {
    fn seal(&self, out: &mut Canonical) {
        out.number(*self);
    }
}
impl Seal for usize {
    fn seal(&self, out: &mut Canonical) {
        out.number(*self as u64);
    }
}
impl Seal for f64 {
    fn seal(&self, out: &mut Canonical) {
        out.number(self.to_bits());
    }
}
impl Seal for Digest {
    fn seal(&self, out: &mut Canonical) {
        let Self { bytes } = self;
        for &byte in bytes {
            out.number(u64::from(byte));
        }
    }
}
