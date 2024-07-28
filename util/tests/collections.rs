use util::collections::HashMap;

#[test]
fn hash_map_test() {
    let mut map = HashMap::new();

    assert_eq!(map.capacity(), 0);

    for i in 0..u16::MAX as u32 {
        assert!(map.insert(format!("{}", i), i).is_none());
    }
    assert!(map.capacity() > u16::MAX as usize);

    for i in 0..u16::MAX as u32 {
        assert_eq!(map.get(&format!("{}", i)).unwrap(), &i);
        *map.get_mut(&format!("{}", i)).unwrap() = 2 * i;
    }
    for i in u16::MAX as u32 + 1..2 * u16::MAX as u32 {
        assert!(map.get(&format!("{}", i)).is_none());
    }

    for i in 0..u16::MAX as u32 {
        assert_eq!(map.insert(format!("{}", i), 3 * i).unwrap(), 2 * i);
    }

    for i in 0..u16::MAX as u32 {
        assert_eq!(map.remove(&format!("{}", i)).unwrap(), 3 * i);
    }
}
