//! These tests don't really assert anything, they just exercise the API.
//! This is primarily intended to be run under miri as a sanitizer.

#![allow(unused)]

use {rc_borrow::ArcBorrow, std::sync::Arc};

#[test]
fn doc_example() {
    type Resource = u128;

    fn acquire_resource() -> Arc<Resource> {
        Arc::new(0)
    }

    fn use_resource(_resource: &Resource) {
        /* ... */
    }

    let resource: Arc<Resource> = acquire_resource();
    let borrowed: ArcBorrow<'_, Resource> = (&resource).into();
    let reference: &Resource = ArcBorrow::downgrade(borrowed);
    let cloned: Arc<Resource> = ArcBorrow::upgrade(borrowed);
    use_resource(&borrowed);
}
