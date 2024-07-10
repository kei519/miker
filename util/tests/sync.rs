use util::sync::OnceStatic;

#[test]
fn test_once_static_init() {
    static VAL: OnceStatic<i32> = OnceStatic::new();
    assert!(!VAL.is_initialized());

    VAL.init(1);
    assert!(VAL.is_initialized());

    assert_eq!(VAL.as_ref(), &1);
    assert_eq!(VAL.get(), 1);

    assert_eq!(unsafe { VAL.as_ref_unchecked() }, &1);
    assert_eq!(unsafe { VAL.get_uncecked() }, 1);
}

#[test]
fn test_once_static_from() {
    static VAL: OnceStatic<u128> = OnceStatic::from(49);
    assert!(VAL.is_initialized());

    assert_eq!(VAL.as_ref(), &49);
    assert_eq!(VAL.get(), 49);
    assert_eq!(unsafe { VAL.as_ref_unchecked() }, &49);
    assert_eq!(unsafe { VAL.get_uncecked() }, 49);
}

#[test]
#[should_panic(expected = "OnceStatic is not initialized")]
fn test_once_static_panic_as_ref() {
    static VAL: OnceStatic<String> = OnceStatic::new();
    VAL.as_ref();
}

#[test]
#[should_panic(expected = "OnceStatic is not initialized")]
fn test_once_static_panic_as_get() {
    static VAL: OnceStatic<char> = OnceStatic::new();
    VAL.get();
}
