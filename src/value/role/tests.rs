// try_from(RoleBit)(positive): singular valid bit should construct a role bit.
// try_from(RoleBit)(negative): composite bits should be rejected for role bit.
// try_from(RoleMask)(positive): valid nonzero mask should construct a role mask.
// try_from(RoleMask)(negative): zero mask should be rejected.
// into(RoleMask)(positive): role mask should convert into raw bits.
// serialize(RoleMask)(positive): role mask should serialize as its raw bit value.
// deserialize(RoleMask)(positive): valid role mask bits should deserialize into a value.
// deserialize(RoleMask)(negative): invalid role mask bits should return the invalid role message.

use super::*;

#[test]
fn try_from_accepts_singular_role_bit() {
    let role_bit = RoleBit::try_from(2).ok().unwrap();

    assert_eq!(role_bit, RoleBit::TRANSLATOR);
}

#[test]
fn try_from_rejects_composite_role_bit() {
    let err = RoleBit::try_from(3).err().unwrap();

    assert_expected_role_error(err);
}

#[test]
fn try_from_accepts_valid_role_mask() {
    let role_mask = RoleMask::try_from(3).ok().unwrap();

    assert_eq!(u32::from(role_mask), 3);
}

#[test]
fn try_from_rejects_zero_role_mask() {
    let err = RoleMask::try_from(0).err().unwrap();

    assert_expected_role_error(err);
}

#[test]
fn into_outputs_raw_bits() {
    let role_mask = RoleMask::from(RoleBit::TRANSLATOR);

    let role_mask_raw = u32::from(role_mask);

    assert_eq!(role_mask_raw, 2);
}

#[test]
fn serialize_outputs_raw_bits() {
    let role_mask = RoleMask::from(RoleBit::TRANSLATOR);

    let role_mask_json = serde_json::to_string(&role_mask).unwrap();

    assert_eq!(role_mask_json, "2");
}

#[test]
fn deserialize_accepts_valid_bits() {
    let role_mask = serde_json::from_str::<RoleMask>("2").unwrap();

    assert_eq!(role_mask, RoleMask::from(RoleBit::TRANSLATOR));
}

#[test]
fn deserialize_rejects_invalid_bits_with_message() {
    let err = serde_json::from_str::<RoleMask>("2147483648")
        .err()
        .unwrap();

    assert!(err.to_string().contains(&trl("error-invalid-role")));
}

fn assert_expected_role_error(err: RootError) {
    let RootError::Expected { message, .. } = err else {
        panic!("expected role error");
    };

    assert_eq!(message, trl("error-invalid-role"));
}
