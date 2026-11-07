use ng_contract::Digest;
use ng_record::{compact, expand};

const START_TIME: usize = 64;
const FIRST_START: usize = 80;

fn link(previous: u8, start: u8, end: u8) -> Vec<u8> {
    let mut link = vec![previous; 32];
    link.extend([9; 32]);
    link.extend([start; 8]);
    link.extend([end; 8]);
    for curve in 0..3 {
        link.extend([start + curve; 16]);
        link.extend([end + curve; 16]);
    }
    link
}

#[test]
fn a_link_that_follows_the_one_before_keeps_its_ends_and_expands_exactly() {
    let first = link(0, 10, 20);
    let kept = compact(None, &first);
    assert_eq!(expand(None, &kept), Some(first));
    let previous = Some((Digest::from_bytes([1; 32]), kept.as_slice()));
    let follows = link(1, 20, 30);
    let compacted = compact(previous, &follows);
    assert_eq!(compacted.len(), 1 + 32 + 8 + 3 * 16);
    assert_eq!(expand(previous, &compacted), Some(follows));
    let next = link(2, 30, 40);
    let after = Some((Digest::from_bytes([2; 32]), compacted.as_slice()));
    assert_eq!(compact(after, &next).len(), compacted.len());
}

#[test]
fn a_link_that_does_not_follow_the_one_before_bit_for_bit_is_kept_whole() {
    let kept = compact(None, &link(0, 10, 20));
    let previous = Some((Digest::from_bytes([1; 32]), kept.as_slice()));
    let follows = link(1, 20, 30);
    let mut moved = follows.clone();
    moved[FIRST_START + 15] ^= 1;
    let mut late = follows.clone();
    late[START_TIME + 7] ^= 1;
    let elsewhere = link(3, 20, 30);
    let mut longer = follows.clone();
    longer.extend([20; 16]);
    for other in [moved, late, elsewhere, longer] {
        let whole = compact(previous, &other);
        assert_eq!(whole.len(), other.len() + 1);
        assert_eq!(expand(previous, &whole), Some(other));
    }
}

#[test]
fn a_compact_link_without_the_link_it_follows_expands_to_nothing() {
    let kept = compact(None, &link(0, 10, 20));
    let previous = Some((Digest::from_bytes([1; 32]), kept.as_slice()));
    let compacted = compact(previous, &link(1, 20, 30));
    assert_eq!(expand(None, &compacted), None);
    let shorter = compact(None, &link(0, 10, 20)[..FIRST_START + 32]);
    assert_eq!(
        expand(Some((Digest::from_bytes([1; 32]), &shorter)), &compacted),
        None
    );
}
