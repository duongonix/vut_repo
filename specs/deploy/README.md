# Vut Deployment Specifications

Deployment and toolchain-distribution specifications for Vut.

These documents define how a Vut installation is assembled, linked, and
distributed. They are subordinate to the language and compiler specifications
in `specs/` but are the source of truth for release/distribution behavior.

| Document | Scope | Status |
| --- | --- | --- |
| [native-linking.md](native-linking.md) | Standalone native linking model, distribution runtime layout, per-OS linkers and system libraries | M-LINK.0 complete: 4 fixtures pass on Windows, Linux, macOS arm64 + x86_64 (default PIE, no rustc/cargo) |

## Convention

The repository keeps two clearly separated goals:

```text
MVP
  no rustc / no cargo in production linking
  platform native SDK/toolchain may be required for the final link

Long-term
  fully standalone Vut toolchain
  no rustc / no cargo
  no external cc/cl where technically and legally practical
```

Every deployment document must state which goal a rule belongs to.
