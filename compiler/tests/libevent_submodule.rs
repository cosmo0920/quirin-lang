use std::path::Path;

use qcore_compiler::event_driven::LIBEVENT_SUBMODULE_PATH;
use qcore_compiler::source_profiles::{LIBEVENT_SOURCE_SET, load_profile};
use qcore_compiler::verify_pinned_libevent_source;

#[test]
fn libevent_submodule_matches_the_audited_revision() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("compiler crate must be inside the repository")
        .join(LIBEVENT_SUBMODULE_PATH);

    // The pinned revision is declared in `contract/profiles/libevent`
    // (W2); this test still asserts the checkout matches it exactly.
    let profile = load_profile(LIBEVENT_SOURCE_SET).expect("libevent source-set profile loads");
    let library = profile
        .library(LIBEVENT_SOURCE_SET)
        .expect("the libevent profile declares its library");
    let pin = profile
        .pin_for(LIBEVENT_SOURCE_SET)
        .expect("the libevent library binds a pin");

    let evidence = verify_pinned_libevent_source(&root, pin, library).unwrap_or_else(|error| {
        panic!(
            "failed to verify the Libevent submodule at {}: {error}\n\
             initialize it with `git submodule update --init --recursive`",
            root.display()
        )
    });

    assert_eq!(evidence.commit, pin.git_commit);
    assert_eq!(evidence.checked_files.len(), library.audited_files.len());
    assert_eq!(
        evidence.checked_witnesses.len(),
        library.source_witnesses.len()
    );
    assert!(
        evidence.checked_files.iter().any(|path| path == "epoll.c"),
        "the pinned source audit must cover the Linux backend"
    );
    assert!(
        evidence.checked_files.iter().any(|path| path == "kqueue.c"),
        "the pinned source audit must cover the native BSD/macOS backend"
    );
    for path in [
        "win32select.c",
        "event_iocp.c",
        "buffer_iocp.c",
        "bufferevent_async.c",
    ] {
        assert!(
            evidence.checked_files.iter().any(|checked| checked == path),
            "the pinned source audit must cover the native Windows contract in {path}"
        );
    }
}
