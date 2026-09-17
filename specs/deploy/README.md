# Vut Deployment Specifications

Deployment and toolchain-distribution specifications for Vut.

These documents define how a Vut installation is assembled, linked, packaged,
and distributed. They are subordinate to the language and compiler
specifications in `specs/` but are the source of truth for release/distribution
behavior.

| Document | Scope | Status |
| --- | --- | --- |
| [native-linking.md](native-linking.md) | Standalone native linking model, per-OS linkers and system libraries | M-LINK.0–6 complete; release links with the platform toolchain (no rustc/cargo) |
| [versioning.md](versioning.md) | Distribution/vpm/runtime-ABI version domains, tags | M2 complete |
| [distribution.md](distribution.md) | Artifact layout, manifest, naming, checksums, `vut-dist` | M4/M5/M15 complete |
| [installer.md](installer.md) | `install.sh` / `install.ps1`, atomic update, PATH | M16–M19 complete |
| [release.md](release.md) | GitHub Actions release pipeline, matrix, cross-repo publishing | M13/M14/M20 complete |

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
