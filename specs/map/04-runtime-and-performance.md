# Runtime and performance

The runtime owns the hash table implementation. Codegen selects versioned ABI calls; backends must not duplicate map algorithms.

Performance contract:

- amortized O(1) lookup, insert, and remove;
- geometric growth;
- `reserve(n)` ensures enough capacity for at least `n` entries;
- no guaranteed iteration order;
- hash function and bucket order may change between processes or versions.

Collision handling must compare hash and equality, so distinct keys with equal hashes do not overwrite each other.

Runtime storage is generic over key/value byte layouts and uses compiler-provided size/alignment metadata. It must not box every element into `dyn`.

Mutation uses COW or an equivalent value-semantic detach when storage is shared.
