## 1. Client

- [x] 1.1 Start a pane press from a left press on a pane frame, deferring focus to the release
- [x] 1.2 Add the `Pane` drop target, excluding the dragged pane itself
- [x] 1.3 Swap on a same-tab drop, open the split chooser on a cross-tab drop
- [x] 1.4 Outline the target pane while dragging

## 2. Verification

- [x] 2.1 Client tests: frame drag swaps and highlights, a frame click still focuses, a drag back onto its own pane sends nothing
- [x] 2.2 Pass `scripts/openspec.sh validate --all --strict`

## 3. Docs and ledger

- [x] 3.1 Describe the drag in `docs/next/website/src/content/docs/concepts.mdx`
- [x] 3.2 Add the `docs/fork/CHANGES.md` entry, with its regression check
