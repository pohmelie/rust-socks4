use socks4::{inc, dec};

#[test]
fn test_inc_dec() {
    assert!(inc(1) == 2);
    assert!(dec(1) == 0);
    assert!(inc(dec(3)) == 3);
}
