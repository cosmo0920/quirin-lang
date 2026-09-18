import pathlib
for card in ["ct42-protocol-cell", "task-route-tsan", "ct-lane-5", "joined-provenance-round-bound",
             "write-lend-tasks-exercise", "mutex-share-cross-emission"]:
    p = pathlib.Path(f"docs/reports/{card}/README.md")
    text = p.read_text(encoding="utf-8")
    old = "\n---\n\n## Published beside by `WAVE-38-CLOSE`"
    new = "\n## Published beside by `WAVE-38-CLOSE`"
    assert text.count(old) == 1, card
    p.write_text(text.replace(old, new), encoding="utf-8")
    print("stripped rule:", card)
