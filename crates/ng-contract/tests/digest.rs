use ng_contract::{Digest, RecordHash};

const FIPS_448: &[u8] = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
const FIPS_896: &[u8] =
    b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu";

fn pattern(length: usize) -> Vec<u8> {
    (0..length).map(|at| (at % 251) as u8).collect()
}

fn hex(text: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (byte, pair) in out.iter_mut().zip(text.as_bytes().as_chunks::<2>().0) {
        let digit = |c: u8| {
            (c as char)
                .to_digit(16)
                .map(|d| d as u8)
                .unwrap_or_else(|| panic!("hex digit {c}"))
        };
        *byte = digit(pair[0]) * 16 + digit(pair[1]);
    }
    out
}

#[test]
fn digest_is_sha256_on_the_fips_180_4_examples() {
    let million = vec![b'a'; 1_000_000];
    let cases: [(&[u8], &str); 5] = [
        (b"", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
        (b"abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
        (
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
        ),
        (
            b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
            "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1",
        ),
        (&million, "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"),
    ];
    for (message, expected) in cases {
        assert_eq!(
            Digest::of_canonical(message).as_bytes(),
            &hex(expected),
            "{} bytes",
            message.len()
        );
    }
}

/// Values from Python 3.14's `hashlib.blake2b(message, digest_size=32)`; GNU coreutils 9.12 `b2sum -l 256` gives
/// the same for each.
#[test]
fn the_record_hash_blake2b_is_rfc_7693_with_a_32_byte_digest() {
    let cases: [(Vec<u8>, &str); 15] = [
        (
            Vec::new(),
            "0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8",
        ),
        (
            b"abc".to_vec(),
            "bddd813c634239723171ef3fee98579b94964e3bb1cb3e427262c8c068d52319",
        ),
        (
            FIPS_448.to_vec(),
            "5f7a93da9c5621583f22e49e8e91a40cbba37536622235a380f434b9f68e49c4",
        ),
        (
            FIPS_896.to_vec(),
            "90a0bcf5e5a67ac1578c2754617994cfc248109275a809a0721feebd1e918738",
        ),
        (
            vec![b'a'; 1_000_000],
            "0741850f36cba4259628355d1073e24ddb9ca0e1bfac36fd39ae5dc2101e23a4",
        ),
        (
            pattern(1),
            "03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314",
        ),
        (
            pattern(104),
            "6efd41538fec50459b6c2583ade5754c86617580eef3071d38723aaf743f93c7",
        ),
        (
            pattern(127),
            "f2fe67ff342e21b8f45e8f2e0bcd1d9243245d50ee6c78042e9c491388791c72",
        ),
        (
            pattern(128),
            "c3582f71ebb2be66fa5dd750f80baae97554f3b015663c8be377cfcb2488c1d1",
        ),
        (
            pattern(129),
            "f7f3c46ba2564ff4c4c162da1f5b605f9f1c4aa6a20652a9f9a337c1a2f5b9c9",
        ),
        (
            pattern(136),
            "6a35d3dadc62dfe7819519f92181b2f8d38f5e0ed3d51a22cf8a133ab628d6f4",
        ),
        (
            pattern(255),
            "d9ef0fc521b4266d16df662bec231bc2ec3989e7adeaf63169c295dc239dbbea",
        ),
        (
            pattern(256),
            "582f782226018ec33076bd8d1c42413530ac7e1126260ffc0f306ba3befc3f24",
        ),
        (
            pattern(257),
            "227e15ed64ee8e93eb7bc53828f76eed974f2c4ab1408c3d08f212b7f8d69904",
        ),
        (
            pattern(1000),
            "b372d0608f720c8c3dd41e9c8eecb10143b41abe520b616607e754bf79c08331",
        ),
    ];
    for (message, expected) in cases {
        assert_eq!(
            Digest::for_record(&message, RecordHash::Blake2b).as_bytes(),
            &hex(expected),
            "{} bytes",
            message.len()
        );
    }
}

#[test]
fn the_record_hash_sha256_is_the_canonical_digest() {
    for message in [&b""[..], b"abc", FIPS_448, FIPS_896, &pattern(129)] {
        assert_eq!(
            Digest::for_record(message, RecordHash::Sha256),
            Digest::of_canonical(message),
            "{} bytes",
            message.len()
        );
    }
}
