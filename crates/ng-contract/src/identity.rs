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

#[derive(Clone, Copy)]
struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    filled: usize,
    total: u64,
}
impl Sha256 {
    fn new() -> Self {
        Self {
            state: INITIAL,
            buffer: [0; 64],
            filled: 0,
            total: 0,
        }
    }
    fn write(&mut self, mut bytes: &[u8]) {
        self.total = self.total.wrapping_add(bytes.len() as u64);
        if self.filled > 0 {
            let take = (64 - self.filled).min(bytes.len());
            self.buffer[self.filled..self.filled + take].copy_from_slice(&bytes[..take]);
            self.filled += take;
            bytes = &bytes[take..];
            if self.filled == 64 {
                compress(&mut self.state, &self.buffer);
                self.filled = 0;
            }
        }
        let (blocks, rest) = bytes.as_chunks::<64>();
        for block in blocks {
            compress(&mut self.state, block);
        }
        if !rest.is_empty() {
            self.buffer[..rest.len()].copy_from_slice(rest);
            self.filled = rest.len();
        }
    }
    #[inline(always)]
    fn write_number(&mut self, value: u64) {
        let bytes = value.to_be_bytes();
        if self.filled < 56 {
            self.buffer[self.filled..self.filled + 8].copy_from_slice(&bytes);
            self.filled += 8;
            self.total = self.total.wrapping_add(8);
        } else {
            self.write(&bytes);
        }
    }
    fn finish(mut self) -> [u8; 32] {
        let bits = self.total.wrapping_mul(8);
        self.buffer[self.filled] = 0x80;
        self.filled += 1;
        if self.filled > 56 {
            self.buffer[self.filled..].fill(0);
            compress(&mut self.state, &self.buffer);
            self.buffer[..56].fill(0);
        } else {
            self.buffer[self.filled..56].fill(0);
        }
        self.buffer[56..64].copy_from_slice(&bits.to_be_bytes());
        compress(&mut self.state, &self.buffer);
        let mut out = [0u8; 32];
        for (bytes, word) in out.as_chunks_mut::<4>().0.iter_mut().zip(self.state) {
            *bytes = word.to_be_bytes();
        }
        out
    }
}

fn sha256(message: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.write(message);
    hasher.finish()
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

#[derive(Clone, Copy)]
struct Blake2b {
    state: [u64; 8],
    buffer: [u8; 128],
    filled: usize,
    compressed: u128,
}
impl Blake2b {
    fn new() -> Self {
        let mut state = BLAKE2B_INITIAL;
        // RFC 7693, section 2.5: depth 1, fanout 1, no key, a 32-byte digest.
        state[0] ^= 0x0101_0020;
        Self {
            state,
            buffer: [0; 128],
            filled: 0,
            compressed: 0,
        }
    }
    #[inline(always)]
    fn compress_buffer(&mut self) {
        self.compressed += 128;
        blake2b_block::<false>(&mut self.state, &self.buffer, self.compressed);
        self.filled = 0;
    }
    fn write(&mut self, mut bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        // RFC 7693 marks the final block with a flag, so a full buffer compresses only when more bytes follow.
        if self.filled == 128 {
            self.compress_buffer();
        }
        if self.filled > 0 {
            let take = (128 - self.filled).min(bytes.len());
            self.buffer[self.filled..self.filled + take].copy_from_slice(&bytes[..take]);
            self.filled += take;
            bytes = &bytes[take..];
            if bytes.is_empty() {
                return;
            }
            self.compress_buffer();
        }
        let before_last = (bytes.len() - 1) / 128;
        let (blocks, _) = bytes.as_chunks::<128>();
        for block in &blocks[..before_last] {
            self.compressed += 128;
            blake2b_block::<false>(&mut self.state, block, self.compressed);
        }
        let tail = &bytes[128 * before_last..];
        self.buffer[..tail.len()].copy_from_slice(tail);
        self.filled = tail.len();
    }
    #[inline(always)]
    fn write_number(&mut self, value: u64) {
        let bytes = value.to_be_bytes();
        if self.filled <= 120 {
            self.buffer[self.filled..self.filled + 8].copy_from_slice(&bytes);
            self.filled += 8;
        } else {
            self.write_straddling(&bytes);
        }
    }
    #[inline(always)]
    fn write_pair(&mut self, first: u64, second: u64) {
        if self.filled <= 112 {
            self.buffer[self.filled..self.filled + 8].copy_from_slice(&first.to_be_bytes());
            self.buffer[self.filled + 8..self.filled + 16].copy_from_slice(&second.to_be_bytes());
            self.filled += 16;
        } else {
            let mut bytes = [0u8; 16];
            bytes[..8].copy_from_slice(&first.to_be_bytes());
            bytes[8..].copy_from_slice(&second.to_be_bytes());
            self.write_straddling(&bytes);
        }
    }
    #[cold]
    #[inline(never)]
    fn write_straddling(&mut self, bytes: &[u8]) {
        let take = 128 - self.filled;
        self.buffer[self.filled..].copy_from_slice(&bytes[..take]);
        self.compress_buffer();
        let rest = &bytes[take..];
        self.buffer[..rest.len()].copy_from_slice(rest);
        self.filled = rest.len();
    }
    fn finish(mut self) -> [u8; 32] {
        self.buffer[self.filled..].fill(0);
        let total = self.compressed + self.filled as u128;
        blake2b_block::<true>(&mut self.state, &self.buffer, total);
        let mut out = [0u8; 32];
        for (bytes, word) in out.as_chunks_mut::<8>().0.iter_mut().zip(self.state) {
            *bytes = word.to_le_bytes();
        }
        out
    }
}

fn blake2b(message: &[u8]) -> [u8; 32] {
    let mut hasher = Blake2b::new();
    hasher.write(message);
    hasher.finish()
}

fn blake2b_block<const LAST: bool>(state: &mut [u64; 8], block: &[u8; 128], counted: u128) {
    let chunks: &[[u8; 8]; 16] = block.as_chunks::<8>().0.try_into().expect("128 bytes");
    let word = |i: usize| u64::from_le_bytes(chunks[i]);
    let mut a0 = [state[0], state[1]];
    let mut a1 = [state[2], state[3]];
    let mut b0 = [state[4], state[5]];
    let mut b1 = [state[6], state[7]];
    let mut c0 = [BLAKE2B_INITIAL[0], BLAKE2B_INITIAL[1]];
    let mut c1 = [BLAKE2B_INITIAL[2], BLAKE2B_INITIAL[3]];
    let mut d0 = [
        BLAKE2B_INITIAL[4] ^ (counted as u64),
        BLAKE2B_INITIAL[5] ^ ((counted >> 64) as u64),
    ];
    let mut d1 = [
        if LAST {
            !BLAKE2B_INITIAL[6]
        } else {
            BLAKE2B_INITIAL[6]
        },
        BLAKE2B_INITIAL[7],
    ];
    macro_rules! run_round {
        ($r:expr) => {{
            let s = &SIGMA[$r];
            mix_pair(
                &mut a0,
                &mut b0,
                &mut c0,
                &mut d0,
                [word(s[0]), word(s[2])],
                [word(s[1]), word(s[3])],
            );
            mix_pair(
                &mut a1,
                &mut b1,
                &mut c1,
                &mut d1,
                [word(s[4]), word(s[6])],
                [word(s[5]), word(s[7])],
            );
            (b0, b1) = ([b0[1], b1[0]], [b1[1], b0[0]]);
            core::mem::swap(&mut c0, &mut c1);
            (d0, d1) = ([d1[1], d0[0]], [d0[1], d1[0]]);
            mix_pair(
                &mut a0,
                &mut b0,
                &mut c0,
                &mut d0,
                [word(s[8]), word(s[10])],
                [word(s[9]), word(s[11])],
            );
            mix_pair(
                &mut a1,
                &mut b1,
                &mut c1,
                &mut d1,
                [word(s[12]), word(s[14])],
                [word(s[13]), word(s[15])],
            );
            (b0, b1) = ([b1[1], b0[0]], [b0[1], b1[0]]);
            core::mem::swap(&mut c0, &mut c1);
            (d0, d1) = ([d0[1], d1[0]], [d1[1], d0[0]]);
        }};
    }
    run_round!(0);
    run_round!(1);
    run_round!(2);
    run_round!(3);
    run_round!(4);
    run_round!(5);
    run_round!(6);
    run_round!(7);
    run_round!(8);
    run_round!(9);
    run_round!(10);
    run_round!(11);
    state[0] ^= a0[0] ^ c0[0];
    state[1] ^= a0[1] ^ c0[1];
    state[2] ^= a1[0] ^ c1[0];
    state[3] ^= a1[1] ^ c1[1];
    state[4] ^= b0[0] ^ d0[0];
    state[5] ^= b0[1] ^ d0[1];
    state[6] ^= b1[0] ^ d1[0];
    state[7] ^= b1[1] ^ d1[1];
}

/// RFC 7693, section 3.1, two independent columns interleaved to expose ILP without exceeding GPR budget.
#[inline(always)]
fn mix_pair(
    a: &mut [u64; 2],
    b: &mut [u64; 2],
    c: &mut [u64; 2],
    d: &mut [u64; 2],
    x: [u64; 2],
    y: [u64; 2],
) {
    a[0] = a[0].wrapping_add(b[0]).wrapping_add(x[0]);
    a[1] = a[1].wrapping_add(b[1]).wrapping_add(x[1]);
    d[0] = (d[0] ^ a[0]).rotate_right(32);
    d[1] = (d[1] ^ a[1]).rotate_right(32);
    c[0] = c[0].wrapping_add(d[0]);
    c[1] = c[1].wrapping_add(d[1]);
    b[0] = (b[0] ^ c[0]).rotate_right(24);
    b[1] = (b[1] ^ c[1]).rotate_right(24);
    a[0] = a[0].wrapping_add(b[0]).wrapping_add(y[0]);
    a[1] = a[1].wrapping_add(b[1]).wrapping_add(y[1]);
    d[0] = (d[0] ^ a[0]).rotate_right(16);
    d[1] = (d[1] ^ a[1]).rotate_right(16);
    c[0] = c[0].wrapping_add(d[0]);
    c[1] = c[1].wrapping_add(d[1]);
    b[0] = (b[0] ^ c[0]).rotate_right(63);
    b[1] = (b[1] ^ c[1]).rotate_right(63);
}

/// `Seal` written straight into a hash, for an identity too large to buffer each step. Each impl writes the bytes the
/// `Seal` of its type writes into a `Canonical`.
pub(crate) mod stream {
    use super::{Blake2b, Digest, RecordHash, Sha256};

    pub(crate) trait Sink {
        fn number(&mut self, number: u64);
        fn raw(&mut self, bytes: &[u8]);
        #[inline(always)]
        fn pair(&mut self, first: u64, second: u64) {
            self.number(first);
            self.number(second);
        }
        fn length(&mut self, length: usize) {
            self.number(length as u64);
        }
        fn text(&mut self, text: &str) {
            self.number(text.len() as u64);
            self.raw(text.as_bytes());
        }
    }
    impl Sink for Sha256 {
        #[inline(always)]
        fn number(&mut self, number: u64) {
            self.write_number(number);
        }
        fn raw(&mut self, bytes: &[u8]) {
            self.write(bytes);
        }
    }
    impl Sink for Blake2b {
        #[inline(always)]
        fn number(&mut self, number: u64) {
            self.write_number(number);
        }
        #[inline(always)]
        fn pair(&mut self, first: u64, second: u64) {
            self.write_pair(first, second);
        }
        fn raw(&mut self, bytes: &[u8]) {
            self.write(bytes);
        }
    }

    pub(crate) trait Seal {
        fn seal<S: Sink>(&self, out: &mut S);
    }
    impl<T: Seal> Seal for [T] {
        fn seal<S: Sink>(&self, out: &mut S) {
            out.length(self.len());
            for item in self {
                item.seal(out);
            }
        }
    }
    impl<T: Seal> Seal for Option<T> {
        fn seal<S: Sink>(&self, out: &mut S) {
            match self {
                None => out.number(0),
                Some(value) => {
                    out.number(1);
                    value.seal(out);
                }
            }
        }
    }
    impl<A: Seal, B: Seal> Seal for (A, B) {
        fn seal<S: Sink>(&self, out: &mut S) {
            self.0.seal(out);
            self.1.seal(out);
        }
    }
    impl Seal for u64 {
        fn seal<S: Sink>(&self, out: &mut S) {
            out.number(*self);
        }
    }
    impl Seal for usize {
        fn seal<S: Sink>(&self, out: &mut S) {
            out.number(*self as u64);
        }
    }
    impl Seal for f64 {
        fn seal<S: Sink>(&self, out: &mut S) {
            out.number(self.to_bits());
        }
    }
    impl Seal for Digest {
        fn seal<S: Sink>(&self, out: &mut S) {
            let Self { bytes } = self;
            for &byte in bytes {
                out.number(u64::from(byte));
            }
        }
    }

    /// The digest `Canonical::new(tag)` then `item.seal` would give under `hash`.
    pub(crate) fn digest(tag: &str, hash: RecordHash, item: &(impl Seal + ?Sized)) -> Digest {
        let bytes = match hash {
            RecordHash::Sha256 => {
                let mut out = Sha256::new();
                out.text(tag);
                item.seal(&mut out);
                out.finish()
            }
            RecordHash::Blake2b => {
                let mut out = Blake2b::new();
                out.text(tag);
                item.seal(&mut out);
                out.finish()
            }
        };
        Digest { bytes }
    }
}

pub(crate) struct Canonical {
    bytes: Vec<u8>,
}
impl Canonical {
    pub(crate) fn new(tag: &str) -> Self {
        let mut canonical = Self {
            bytes: Vec::with_capacity(256),
        };
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
    pub(crate) fn digest_for(&self, hash: RecordHash) -> Digest {
        Digest::for_record(&self.bytes, hash)
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

#[cfg(test)]
mod tests {
    use super::{
        BLAKE2B_INITIAL, Blake2b, Canonical, Digest, INITIAL, RecordHash, Seal, Sha256, blake2b_block,
        compress, stream,
    };
    use crate::{Compensated, SimTime};

    fn oneshot_sha256(message: &[u8]) -> [u8; 32] {
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

    fn oneshot_blake2b(message: &[u8]) -> [u8; 32] {
        let mut state = BLAKE2B_INITIAL;
        state[0] ^= 0x0101_0020;
        let before_last = message.len().saturating_sub(1) / 128;
        let (blocks, _) = message.as_chunks::<128>();
        for (at, block) in blocks[..before_last].iter().enumerate() {
            blake2b_block::<false>(&mut state, block, 128 * (at as u128 + 1));
        }
        let tail = &message[128 * before_last..];
        let mut last = [0u8; 128];
        last[..tail.len()].copy_from_slice(tail);
        blake2b_block::<true>(&mut state, &last, message.len() as u128);
        let mut out = [0u8; 32];
        for (bytes, word) in out.as_chunks_mut::<8>().0.iter_mut().zip(state) {
            *bytes = word.to_le_bytes();
        }
        out
    }

    #[test]
    fn streaming_sha256_and_blake2b_match_oneshot_across_block_boundaries() {
        let lengths = [
            0, 1, 7, 8, 54, 55, 56, 57, 63, 64, 65, 119, 120, 127, 128, 129, 191, 192, 255, 256, 257, 512,
            1300,
        ];
        let chunks = [1, 3, 7, 8, 13, 55, 64, 127, 128, 200];
        for length in lengths {
            let data: Vec<u8> = (0..length).map(|i| ((i * 37 + 11) & 0xff) as u8).collect();
            let expected_sha = oneshot_sha256(&data);
            let expected_b2 = oneshot_blake2b(&data);
            assert_eq!(super::sha256(&data), expected_sha, "oneshot sha len={length}");
            assert_eq!(super::blake2b(&data), expected_b2, "oneshot b2 len={length}");
            for step in chunks {
                let mut sha = Sha256::new();
                let mut b2 = Blake2b::new();
                sha.write(&[]);
                b2.write(&[]);
                for part in data.chunks(step) {
                    sha.write(part);
                    b2.write(part);
                    sha.write(&[]);
                    b2.write(&[]);
                }
                assert_eq!(sha.finish(), expected_sha, "stream sha len={length} step={step}");
                assert_eq!(b2.finish(), expected_b2, "stream b2 len={length} step={step}");
            }
        }
    }

    #[test]
    fn streaming_canonical_matches_buffered_across_inline_and_heap_sizes() {
        struct Sample {
            values: Vec<Compensated>,
            floats: Vec<f64>,
            time: SimTime,
            digest: Digest,
        }
        impl stream::Seal for Sample {
            fn seal<S: stream::Sink>(&self, out: &mut S) {
                let Self {
                    values,
                    floats,
                    time,
                    digest,
                } = self;
                stream::Seal::seal(values.as_slice(), out);
                stream::Seal::seal(floats.as_slice(), out);
                stream::Seal::seal(time, out);
                stream::Seal::seal(digest, out);
            }
        }
        for count in [0, 1, 7, 16, 80, 400] {
            let sample = Sample {
                values: (0..count)
                    .map(|i| Compensated::exact(i as f64 * 0.25 - 3.5))
                    .collect(),
                floats: (0..count).map(|i| -(i as f64) * 1.5).collect(),
                time: SimTime::from_ns(count as u64 * 1_000_000 + 42),
                digest: Digest::of_canonical(b"seed"),
            };
            let mut buffered = Canonical::new("ng-contract sample 1");
            Seal::seal(sample.values.as_slice(), &mut buffered);
            Seal::seal(sample.floats.as_slice(), &mut buffered);
            Seal::seal(&sample.time, &mut buffered);
            Seal::seal(&sample.digest, &mut buffered);
            let bytes = buffered.into_bytes();
            for hash in [RecordHash::Sha256, RecordHash::Blake2b] {
                assert_eq!(
                    stream::digest("ng-contract sample 1", hash, &sample),
                    Digest::for_record(&bytes, hash),
                    "count={count} hash={hash:?}"
                );
            }
        }
    }

    #[test]
    fn blake2b_write_number_and_pair_match_oneshot_at_every_buffer_offset() {
        for prefix_len in 0..=128 {
            let prefix: Vec<u8> = (0..prefix_len).map(|i| (i * 19 + 5) as u8).collect();
            let a = 0x0123_4567_89ab_cdef_u64;
            let b = 0xfedc_ba98_7654_3210_u64;
            let c = 0x1122_3344_5566_7788_u64;
            let mut full = prefix.clone();
            full.extend_from_slice(&a.to_be_bytes());
            full.extend_from_slice(&b.to_be_bytes());
            full.extend_from_slice(&c.to_be_bytes());
            let expected = oneshot_blake2b(&full);

            let mut h1 = Blake2b::new();
            h1.write(&prefix);
            h1.write_pair(a, b);
            h1.write_number(c);
            assert_eq!(h1.finish(), expected, "pair+number prefix={prefix_len}");

            let mut h2 = Blake2b::new();
            h2.write(&prefix);
            h2.write_number(a);
            h2.write_pair(b, c);
            assert_eq!(h2.finish(), expected, "number+pair prefix={prefix_len}");
        }
        // RFC 7693 / BLAKE2b-256("abc")
        let expected_abc = [
            0xbd, 0xdd, 0x81, 0x3c, 0x63, 0x42, 0x39, 0x72, 0x31, 0x71, 0xef, 0x3f, 0xee, 0x98, 0x57, 0x9b,
            0x94, 0x96, 0x4e, 0x3b, 0xb1, 0xcb, 0x3e, 0x42, 0x72, 0x62, 0xc8, 0xc0, 0x68, 0xd5, 0x23, 0x19,
        ];
        assert_eq!(super::blake2b(b"abc"), expected_abc);
    }
}
