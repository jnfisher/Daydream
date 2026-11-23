//! Laying out a B-tree from sorted entries, for the `.DAT` writer (Daydream addition).
//!
//! Btrieve's index is a B-tree, not a B+ tree: inner pages hold entries too, and a page of
//! `n` entries has `n + 1` children (one before each entry and the last-child pointer).
//! Btrieve grows its trees one insert at a time; a rebuilt file is laid out at once from
//! the sorted entries instead. Every page but the root holds at least half a page of
//! entries (Btrieve's own minimum, the key definition's "half" field), so the engine can
//! insert into and delete from the tree afterwards as it would into one it grew.

/// One index page of the laid-out tree: the entries it holds (as indexes into the sorted
/// entries) and its children (as indexes into [`Tree::pages`]); `children` is empty for a
/// leaf and otherwise one longer than `entries`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Page {
    pub entries: Vec<usize>,
    pub children: Vec<usize>,
}

/// A laid-out tree; `pages[root]` is the root. An empty index is one empty leaf, as
/// Btrieve creates it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tree {
    pub pages: Vec<Page>,
    pub root: usize,
}

/// The most entries a tree of `depth` levels of pages holding `max` entries can take.
fn capacity(max: usize, depth: u32) -> usize {
    (max + 1).saturating_pow(depth).saturating_sub(1)
}

/// Lays out `n` sorted entries in pages of at most `max` entries.
pub(crate) fn layout(n: usize, max: usize) -> Tree {
    assert!(max >= 2, "an index page holds at least two entries");
    let mut depth = 1;
    while capacity(max, depth) < n {
        depth += 1;
    }
    let mut pages = Vec::new();
    let root = build(&mut pages, 0, n, depth, max);
    Tree { pages, root }
}

/// Builds the subtree of entries `first..first + n` with `depth` levels; returns its page.
fn build(pages: &mut Vec<Page>, first: usize, n: usize, depth: u32, max: usize) -> usize {
    if depth == 1 {
        debug_assert!(n <= max);
        pages.push(Page {
            entries: (first..first + n).collect(),
            children: Vec::new(),
        });
        return pages.len() - 1;
    }
    let below = capacity(max, depth - 1);
    // the fewest entries on this page that leave every child within its capacity
    let k = n.saturating_sub(below).div_ceil(below + 1).max(1);
    let k = k.min(n);
    let children_total = n - k;
    let per = children_total / (k + 1);
    let extra = children_total % (k + 1);
    let mut entries = Vec::with_capacity(k);
    let mut children = Vec::with_capacity(k + 1);
    let mut at = first;
    for c in 0..=k {
        let size = per + usize::from(c < extra);
        children.push(build(pages, at, size, depth - 1, max));
        at += size;
        if c < k {
            entries.push(at);
            at += 1;
        }
    }
    pages.push(Page { entries, children });
    pages.len() - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every entry once, in order, by an in-order walk; every page but the root at least
    /// half full and at most full; every leaf at the same depth.
    fn check(tree: &Tree, n: usize, max: usize) {
        fn walk(t: &Tree, p: usize, depth: usize, out: &mut Vec<usize>, leaves: &mut Vec<usize>) {
            let page = &t.pages[p];
            if page.children.is_empty() {
                out.extend(&page.entries);
                leaves.push(depth);
                return;
            }
            assert_eq!(page.children.len(), page.entries.len() + 1);
            for (i, &child) in page.children.iter().enumerate() {
                walk(t, child, depth + 1, out, leaves);
                if let Some(&e) = page.entries.get(i) {
                    out.push(e);
                }
            }
        }
        let (mut out, mut leaves) = (Vec::new(), Vec::new());
        walk(tree, tree.root, 1, &mut out, &mut leaves);
        assert_eq!(out, (0..n).collect::<Vec<_>>());
        assert!(
            leaves.windows(2).all(|w| w[0] == w[1]),
            "leaves at one depth"
        );
        for (i, p) in tree.pages.iter().enumerate() {
            assert!(p.entries.len() <= max);
            if i != tree.root {
                assert!(
                    p.entries.len() >= max / 2,
                    "page {i}: {} of {max}",
                    p.entries.len()
                );
            }
        }
    }

    #[test]
    fn trees_of_every_size_are_valid() {
        for max in [2, 3, 4, 11, 41] {
            for n in 0..2500 {
                check(&layout(n, max), n, max);
            }
        }
    }

    #[test]
    fn an_empty_index_is_one_empty_leaf() {
        let t = layout(0, 50);
        assert_eq!(
            t.pages,
            vec![Page {
                entries: vec![],
                children: vec![]
            }]
        );
    }

    #[test]
    fn a_large_index_is_shallow() {
        // the stock rooms file: 26,694 records, 4096-byte pages of 10-byte entries
        let t = layout(26_694, 408);
        let depth = {
            let (mut d, mut p) = (1, t.root);
            while let Some(&c) = t.pages[p].children.first() {
                d += 1;
                p = c;
            }
            d
        };
        assert_eq!(depth, 2);
    }
}
