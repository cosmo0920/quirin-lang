use super::*;

#[test]
fn logger_levels_have_the_complete_ordered_vocabulary() {
    let levels = [
        Level::Trace,
        Level::Debug,
        Level::Info,
        Level::Warn,
        Level::Error,
        Level::Fatal,
    ];
    let keywords = levels.map(Level::keyword);
    let tags = levels.map(Level::tag);
    assert_eq!(
        keywords,
        ["trace", "debug", "info", "warn", "error", "fatal"]
    );
    assert_eq!(tags, ["TRACE", "DEBUG", "INFO", "WARN", "ERROR", "FATAL"]);
    for (index, keyword) in keywords.into_iter().enumerate() {
        assert_eq!(Level::parse(keyword), Some(levels[index]));
    }
    assert!(levels.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn streams_compose_pointwise_and_have_an_identity() {
    let left =
        Streams::singleton(Stream::Out, "one").compose(Streams::singleton(Stream::Err, "warning"));
    let right =
        Streams::singleton(Stream::Out, "two").compose(Streams::singleton(Stream::Err, "failure"));
    let combined = left.clone().compose(right.clone());

    assert_eq!(combined.project(Stream::Out), ["one", "two"]);
    assert_eq!(combined.project(Stream::Err), ["warning", "failure"]);
    assert_eq!(Streams::empty().compose(left.clone()), left);
    assert_eq!(right.clone().compose(Streams::empty()), right);
}

#[test]
fn streams_composition_is_associative() {
    let a = Streams::singleton(Stream::Out, "a");
    let b = Streams::singleton(Stream::Err, "b");
    let c = Streams::singleton(Stream::Out, "c");
    assert_eq!(
        a.clone().compose(b.clone()).compose(c.clone()),
        a.compose(b.compose(c))
    );
}
