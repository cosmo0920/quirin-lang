use std::path::Path;

use qcore_compiler::event_driven::LIBEVENT_SUBMODULE_PATH;
use qcore_compiler::{PINNED_LIBEVENT_COMMIT, verify_pinned_libevent_source};

#[test]
fn libevent_submodule_matches_the_audited_revision() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("compiler crate must be inside the repository")
        .join(LIBEVENT_SUBMODULE_PATH);

    let evidence = verify_pinned_libevent_source(&root).unwrap_or_else(|error| {
        panic!(
            "failed to verify the Libevent submodule at {}: {error}\n\
             initialize it with `git submodule update --init --recursive`",
            root.display()
        )
    });

    assert_eq!(evidence.commit, PINNED_LIBEVENT_COMMIT);
    assert!(
        evidence.checked_files.iter().any(|path| path == "epoll.c"),
        "the pinned source audit must cover the Linux backend"
    );
    assert!(
        evidence.checked_files.iter().any(|path| path == "kqueue.c"),
        "the pinned source audit must cover the native BSD/macOS backend"
    );
}
