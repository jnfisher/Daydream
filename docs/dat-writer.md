# Writing `WCC*.DAT` directly: the M1 spike

The approach doc aims for direct `.DAT` writes, with NMR's `.mdb` import kept as the
fallback. This spike asks how far a direct writer can go today, and what it would take to
go further.

## What works now

**Editing existing records in place.** `btrieve::update::update_in_place` overwrites
records in their slots. It does not touch the index pages, the FCR, the PAT, the slots'
usage counts or their duplicate-chain pointers.

That is sound only while no key changes, so an update that changes any key byte is
refused. Every update is checked before anything is written. In the MajorMUD files the
keys are record numbers and a few names, so this covers editing any existing room,
item, monster, spell, shop, class, race or message. It does not cover renumbering one.

**Checking a file's indexes.** `BtrieveFile::verify_indexes` walks every key's index
tree and checks it against the records:

- every live record is in each index exactly once, under the key bytes it holds;
- the entries are in key order;
- the entry count matches the key definition's.

It passes on the in-repo fixtures and on all eight of wbtrv32's sample files. Those
include v5 and v6 files, two-level trees, linked and repeating duplicates, and segmented
keys. CI also runs it on every stock MajorMUD file. The tests rewrite every non-key byte
of several files (stock items, monsters, rooms and spells, and wbtrv32's v6 samples),
then check that only those bytes changed and that the indexes still match.

## The index format

Nothing public documents this format, and wbtrv32 never reads it, because it rebuilds
indexes from the records. The layout below was decoded from the sample files. It is
written up in `crates/daydream-wcc/src/btrieve/index.rs`:

- **Key definitions.** A key's first segment definition starts with its root page and
  its entry count.
- **Index pages.** Each index page holds:
  - an 8-byte header (page id, a usage count, the entry count);
  - a pointer to the child page holding keys above the last entry;
  - the entries in key order. Each entry is a pointer to the child page of smaller
    keys, the key bytes, and the record's byte address. A key with linked duplicates
    also stores the address of the last record in its chain.
- **B-tree, not B+ tree.** Inner pages hold real entries too. A leaf has no child
  pointers.
- **Linked duplicates.** Each record keeps a previous and next pointer per key with
  linked duplicates, at the end of its slot.
- **Splits.** Btrieve splits a full page in half. For example, a 408-entry leaf leaves
  two 204-entry leaves.
- **Zstring keys.** A zstring key is copied into the index only up to its NUL.

## What a full writer still needs

Adding, deleting or renumbering records means changing the trees: inserting and
removing entries, splitting and merging pages, and allocating pages. On v6 files it also
means keeping the PAT and the shadow-paging usage counts consistent. The checker gives a
precise test for the result.

The open question is acceptance. A tree that passes our checker may still not be one
that Btrieve accepts, for example if it trips on a usage count or a free-page list that
we have not decoded. The only real test is Btrieve itself, meaning Pervasive's
`WBTRV32.DLL` as NMR and Worldgroup use it. It is not freely redistributable, so it
cannot run in public CI. Real Btrieve could check a written file in one of two ways:

- a test on a machine that has it;
- the differential job with real NMR, if that machine has Btrieve.

## Recommendation

1. Use in-place updates for edits to existing records. They need no index work, and
   the indexes stay valid by construction.
2. Keep NMR's `.mdb` import as the path for new and renumbered records until a
   tree-writing version passes against real Btrieve.
3. Next, write the tree inserts and deletes against the checker. Then validate them
   with real Btrieve as part of the NMR differential job.

Not covered yet: variable-length files (`WCCTEXT`, whose text lives in fragment pages)
are refused by the writer. Changing their text in place, or growing it, needs the
fragment pages maintained.
