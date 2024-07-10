use std::fmt::Write as _;

use util::buffer::StrBuf;

#[test]
fn buf_str_test() {
    let mut buf = [0; 1024];
    let mut buf = StrBuf::new(&mut buf);
    assert!(buf.to_str().is_empty());

    write!(buf, "{:08x}", 0x4928).unwrap();
    assert_eq!(buf.to_str(), "00004928");

    writeln!(buf, "Hello World!").unwrap();
    assert_eq!(buf.to_str(), "00004928Hello World!\n");

    let mut buf = [0; 1];
    let mut buf = StrBuf::new(&mut buf);

    write!(buf, "a").unwrap();
    assert_eq!(buf.to_str(), "a");

    assert!(write!(buf, "b").is_err());
    assert_eq!(buf.to_str(), "a");

    let mut buf = [0; 1];
    let mut buf = StrBuf::new(&mut buf);

    assert!(write!(buf, "cd").is_err());
    assert_eq!(buf.to_str(), "c");
}
