use ng_contract::Digest;

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
