use vut_runtime::abi;

#[test]
fn bounds_operation_names_are_stable() {
    assert_eq!(abi::bounds_op::name(abi::bounds_op::LIST_AT), "list.at");
    assert_eq!(abi::bounds_op::name(abi::bounds_op::LIST_SET), "list.set");
    assert_eq!(
        abi::bounds_op::name(abi::bounds_op::LIST_INSERT),
        "list.insert"
    );
    assert_eq!(
        abi::bounds_op::name(abi::bounds_op::LIST_REMOVE),
        "list.remove"
    );
    assert_eq!(abi::bounds_op::name(abi::bounds_op::BYTES_AT), "bytes.at");
    assert_eq!(abi::bounds_op::name(abi::bounds_op::BYTES_SET), "bytes.set");
    assert_eq!(abi::bounds_op::name(abi::bounds_op::ARRAY_AT), "array.at");
    assert_eq!(abi::bounds_op::name(abi::bounds_op::ARRAY_SET), "array.set");
    assert_eq!(
        abi::bounds_op::name(abi::bounds_op::ARRAY_FIRST),
        "array.first"
    );
    assert_eq!(
        abi::bounds_op::name(abi::bounds_op::ARRAY_LAST),
        "array.last"
    );
}

#[test]
fn signed_index_recovers_negative_values() {
    assert_eq!(abi::signed_index(5), 5);
    assert_eq!(abi::signed_index(usize::MAX), -1);
    assert_eq!(abi::signed_index(usize::MAX - 1), -2);
}
