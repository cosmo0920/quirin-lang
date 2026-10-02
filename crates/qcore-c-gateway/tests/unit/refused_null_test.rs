//! Card MONKEY-VHOST-PROPERTY-CUSTODY: rule 12e's null-edge step and its
//! mechanical binding to `proof/QCore/RefusedNullTest.lean`.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/RefusedNullTest.lean");

#[test]
fn the_null_edge_vectors_hold() {
    for &((denoted, nullable, live), step) in NULL_EDGE_VECTORS {
        assert_eq!(null_edge_step(denoted, nullable, live).code(), step);
    }
}

#[test]
fn the_vectors_are_every_combination() {
    assert_eq!(NULL_EDGE_VECTORS.len(), 8);
    for bits in 0u8..8 {
        let key = (bits & 4 != 0, bits & 2 != 0, bits & 1 != 0);
        assert!(NULL_EDGE_VECTORS.iter().any(|(row, _)| *row == key));
    }
}

#[test]
fn the_lean_vectors_are_the_rust_table() {
    let table = LEAN
        .split("def nullEdgeVectors")
        .nth(1)
        .expect("table header");
    let table = &table[..table.find("\n]").expect("end of table")];
    let rows: Vec<Vec<String>> = table
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|row| {
            row.trim_end_matches(',')
                .replace(['(', ')'], "")
                .split(',')
                .map(|cell| cell.trim().trim_start_matches('.').to_owned())
                .collect()
        })
        .collect();
    assert_eq!(rows.len(), NULL_EDGE_VECTORS.len());
    for (row, &((d, n, l), step)) in rows.iter().zip(NULL_EDGE_VECTORS) {
        let cells: Vec<bool> = row[..3].iter().map(|cell| cell == "true").collect();
        assert_eq!(cells, [d, n, l]);
        assert_eq!(row[3], step);
    }
}
