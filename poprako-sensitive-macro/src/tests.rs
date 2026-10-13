//! Compile and run both assertion modes without rebuilding the server.

mod compile;

use quote::quote;

use crate::debug::expand;

#[test]
fn invalid_markers_and_unsupported_items_are_rejected() {
    //
    for input in [
        quote!(
            struct Invalid {
                #[sensitive(always)]
                secret: String,
            }
        ),
        quote!(
            struct Invalid {
                #[sensitive = true]
                secret: String,
            }
        ),
        quote!(
            struct Invalid {
                #[sensitive]
                #[sensitive]
                secret: String,
            }
        ),
        quote!(
            #[sensitive]
            struct Invalid {
                secret: String,
            }
        ),
        quote!(
            enum Invalid {
                Value,
            }
        ),
        quote!(union Invalid { value: u32 }),
    ] {
        assert!(expand(input).is_err());
    }
}
