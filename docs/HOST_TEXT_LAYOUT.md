# Host text layout

`ratex-layout` supports an optional borrowed `TextLayout` callback on
`LayoutOptions`. It lets an embedding application use its own font selection,
Unicode shaping and outline generation **before** RaTeX places scripts,
fractions, arrows, array cells or proof rules. It does not change the parser,
bundled fonts or DisplayList JSON protocol.

```rust,ignore
use ratex_layout::{layout, LayoutOptions, TextLayout};

// `host` implements TextLayout and lives for this synchronous layout call.
let options = LayoutOptions {
    text_layout: Some(&host),
    ..LayoutOptions::default()
};
let measured = layout(&ast, &options);
```

## Callback contract

- The callback receives text-mode AST slices, including `Text` bodies and
  adjacent literal leaves/groups. Unsupported compound bodies may be offered
  before their constituent text runs. Return `None` to use normal recursive
  layout; mathematical alphabet commands keep their existing math-font path.
- Return a `LayoutBox` with finite width, ascent (`height`) and descent (`depth`)
  in **local em units**. RaTeX applies script and explicit TeX size scaling when
  positioning the box; the callback must not apply those multipliers twice.
- Outline coordinates in `BoxContent::SvgPath` start at the baseline-left
  origin. Positive x points right, positive y down, so ascenders have negative
  y. Keep ink extents consistent with the returned dimensions.
- Use `options.color` for the returned box. `text_weight` and `text_italic`
  preserve nested text-command state; `None` selects the host default. These
  fields survive style, color and tracking derivation. `textnormal` resets both.
- The hook can be called more than once for a run, including two-pass delimiter
  layout. It must not rely on a single invocation. Hosts may record diagnostics
  with interior mutability, but do not re-enter layout on the same AST slice.
- Font lookup, missing-glyph policy, text normalization, bidi, shaping and
  outline generation remain the host's responsibility. RaTeX does not invent
  fallback fonts or substitute text after placement.

With no callback, or a callback that always returns `None`, existing bundled
font output is retained. New callers can keep using `LayoutOptions::default()`
and struct updates; stored options now carry the callback lifetime, for example
`LayoutOptions<'a>`. Existing language/platform wrappers need no callback.

Parser-produced ASTs retain the [shared depth policy](STACK_SAFETY.md). The hook
does not introduce a separate parser or depth allowance. Hosts traversing AST
groups should observe that policy and avoid unbounded recursive queries.

## Verification

```bash
cargo test -p ratex-layout
cargo test --release -p ratex-layout --test host_text
cargo test --release -p ratex-layout --test stack_safety
```

The host-text tests check adjacent Unicode runs, measured script placement,
nested weight/slant/color, chemistry and proof cells, unchanged output without
a host, and the accepted/rejected depth boundaries on a 512 KiB release stack.
