#[test]
fn core_crate_builds_and_links() {
    assert_eq!(passero_core::version_tag(), "passero-core");
}
