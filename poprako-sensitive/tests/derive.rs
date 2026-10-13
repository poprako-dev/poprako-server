//! Public import and ordinary Debug composition contracts.

use poprako_sensitive::{Sensitive, SensitiveDebug};

#[derive(SensitiveDebug)]
struct Login<'a> {
    user_id: &'a str,

    #[sensitive]
    password: &'a str,
}

#[derive(Debug)]
struct Request<'a> {
    login: Login<'a>,
}

#[test]
fn derive_reexport_composes_with_standard_debug() {
    //
    let request = Request {
        login: Login {
            user_id: "safe-id",
            password: "private-password",
        },
    };

    assert_eq!(request.login.user_id, "safe-id");

    let rendered = format!("{request:#?}");

    assert!(rendered.contains("safe-id"));

    assert_eq!(
        rendered.contains("private-password"),
        cfg!(debug_assertions)
    );

    assert_eq!(rendered.contains("[REDACTED]"), !cfg!(debug_assertions));
}

#[test]
fn borrowed_wrapper_preserves_original_values() {
    //
    let password = String::from("private-password");

    let sensitive = Sensitive::new(&password);

    assert_eq!(
        format!("{sensitive:?}").contains(&password),
        cfg!(debug_assertions)
    );

    assert_eq!(
        format!("{sensitive}").contains(&password),
        cfg!(debug_assertions)
    );

    assert_eq!(password, "private-password");
}
