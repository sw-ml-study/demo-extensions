# Hugging Face tokenizer extension (`_hftok`)

A byte-level BPE encoder and decoder that reads a Hugging Face `tokenizer.json`
and turns text into token ids. It is a file-format and throughput service. It
knows nothing about chat roles, prompts, conversations, or models.

Work order: `../reasoning-from-scratch/docs/demo-extensions-requests.md` E1,
which is the same need the upstream triage records as RS11.

## Why this is native

Two reasons, both from the work order.

A `tokenizer.json` is a format, not a computation: a vocabulary, a merge list,
an added-token table, and a pre-tokenization pattern that must be interpreted
exactly as the file specifies. Parsing it is ordinary library work that every
ecosystem ships.

Encoding 12,000 training prompts is throughput work. Byte-level BPE applies a
ranked merge list repeatedly over each pre-token, which is pointer-chasing over
small string fragments rather than array math, and the interpreter has no
representation that makes it cheap.

Nothing here differentiates, so by the rule in
`../reasoning-from-scratch/docs/feature-homes.md` it is extension work.

## What stays in MLPL

Chat templating. The extension never sees a role, a system prompt, or a
conversation. `lib/tokenizer/template.mlpl` in the consuming repository
composes control token ids into a prompt; this extension only reports which id
each control token has. The end-of-sequence rule, think-tag handling, and every
decision about what to encode also stay there.

## Surface

Private namespace `_hftok`, public facade `hftok`.

| Call | Returns |
|---|---|
| `_hftok:load(path)` | opaque handle |
| `_hftok:encode(handle, text)` | integer array of ids |
| `_hftok:decode(handle, ids)` | string, control tokens still visible |
| `_hftok:token_to_id(handle, token)` | integer, or `err` when absent |
| `_hftok:info(handle)` | record, described below |

`info` reports `vocabulary_size`, the ids of the end-of-text, turn-start,
turn-end, and think tokens, and the `pattern` string the file itself specifies.
A control token the file does not define is reported as `-1` rather than
omitted, so the record shape is fixed.

Handles are typed and generational. A stale handle, a handle from another
extension, and a handle of the wrong type all fail closed as `err`.

### Root confinement

The work order asks for a one-argument `load(path)` reading beneath "the
configured sandbox root". This repository has no ambient root configuration and
will not invent one, because an ambient root is exactly the kind of implicit
authority the extension rules reject. The private surface therefore takes an
explicit root, matching the SQLite and download surfaces:

```text
_hftok:load({root: <absolute existing directory>, path: <confined relative path>})
```

The public facade preserves the requested one-argument spelling by binding the
root explicitly:

```mlpl
u:hftok_load_in(root, path)   # explicit, the honest primitive
u:hftok_load(path)            # one-argument, resolves against a root the caller set
```

The resolved target must still lie beneath the canonical root after symbolic
links are resolved.

## Accepted files

The extension accepts only what it can encode correctly, and says so rather
than guessing:

- `model.type` is `BPE`.
- The decoder is `ByteLevel`.
- The pre-tokenizer is either a bare `ByteLevel`, or a `Sequence` whose members
  are a `Split` carrying the pattern and a `ByteLevel`. Real vocabularies use
  the `Sequence` form, so both are supported.
- `model.merges` entries are either two-element arrays or single strings with
  one space between halves. Current files use arrays; older ones use strings.

Anything else, including a `WordPiece` or `Unigram` model, a missing vocabulary
or merge list, a normalizer that would rewrite text before encoding, or
malformed JSON, is an `err` with a reason. Nothing panics.

## Error taxonomy

Invalid-argument errors: a malformed `load` record, a relative root, an empty,
absolute or traversing path, a path that escapes the root or is not a regular
file, a wrong argument type, and a stale or foreign handle. Extension errors:
a file that cannot be read, JSON that will not parse, an unsupported model
type, pre-tokenizer or decoder, and a missing or malformed section. Every
failure is a value, never a panic.

## Fixture

`fixtures/tokenizer/tiny-tokenizer.json` is a synthetic file in genuine Hugging
Face format: a small byte-level vocabulary, a real merge list, the same
pre-tokenization pattern real vocabularies use, and the five control tokens the
work order names. `fixtures/tokenizer/tiny-expected-encodings.json` pins the
ids each sample text must produce.

The fixture is owned here so this repository can develop and test without
waiting on the consuming repository, which will publish its own fixture and
reference encoder. Reconciling the two, and running the real Qwen3 goldens, is
a later step; until then no claim is made that the two agree.

## The algorithm

Encoding runs in four stages, and each stage uses only what the file supplies.

1. **Added-token isolation.** Added tokens, which include the five control
   tokens, are matched literally and longest-first, so `<|im_start|>` wins over
   any shorter prefix. Each match becomes its own id and never merges with
   neighbouring text. The text between matches continues to the next stage.
2. **Pre-tokenization.** The file's own pattern splits text into fragments. A
   file with a bare `ByteLevel` pre-tokenizer and no pattern treats the whole
   segment as one fragment.
3. **Byte-level mapping.** Each fragment's UTF-8 bytes map through the GPT-2
   byte-to-unicode alphabet, so all 256 byte values become printable code
   points. This is why a leading space appears as `Ġ`. Arbitrary bytes survive
   a text vocabulary because of this mapping, and non-ASCII text encodes as its
   bytes even when no merge covers it.
4. **Merging.** The merge list is ranked, not ordered by position. Each pass
   finds the lowest-ranked adjacent pair anywhere in the fragment and merges
   it, repeating until no pair is mergeable. A left-to-right pass would give
   different, wrong output: in the fixture `the` must become `t` + `he`,
   because `(h,e)` is a merge and `(t,h)` is not.

Decoding reverses this. Added tokens decode to their literal spelling, so
control tokens stay visible rather than being dropped. Every other token's
glyphs map back to bytes, and the byte string is validated as UTF-8.

### Regular expressions

Pre-tokenization uses `fancy-regex`, because the pattern real vocabularies
carry contains a negative lookahead (`\s+(?!\S)`) that the `regex` crate
cannot compile. `fancy-regex` is pure Rust, so this adds no C `onig` build.

This is the only regex use in the repository, and it is internal. No
general-purpose regex primitive is exposed; the downstream work order lists
regular expressions under "not requested".

## Limits

- **A pattern that does not compile is refused** when the tokenizer is built,
  with the reason, rather than panicking later.
- **Pre-tokenization must cover its input.** If the pattern's matches leave a
  gap, or the pattern fails at run time, encoding is an error. Encoding a
  silently truncated prefix would be worse than refusing: the caller would
  receive plausible ids for text it never supplied.
- **A fragment absent from the vocabulary is an error.** With a complete
  byte-level alphabet this cannot happen; with an incomplete one it is
  reported rather than guessed at.
- **Decoding validates UTF-8.** A partial multi-byte sequence, for instance one
  id of the two that spell `é`, is an error rather than replacement characters.
- **No unknown-token fallback.** `unk_token`, `byte_fallback`, `dropout`, and
  `ignore_merges` are not honoured; files relying on them are outside the
  supported set described above.

## Status

Delivered: the contract above, the crate, file parsing and validation, the
fixture, and byte-level encoding and decoding. Typed handles and the MLPL
facade follow in the next step.
