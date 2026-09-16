---
title: Installation
description: 'Prepare your toolchain and check that Vut is ready to use.'
section: Getting started
---

<script>
  import ContentTabs from '$lib/components/docs/ContentTabs.svelte';
  const platforms = [
    { label: 'Windows', text: 'Use PowerShell or Windows Terminal. Ensure the folder containing vut.exe is on PATH, then reopen the terminal after changing PATH.' },
    { label: 'macOS', text: 'Use Terminal with a macOS-compatible toolchain. Ensure its executable directory is on PATH and that the binary matches your CPU architecture.' },
    { label: 'Linux', text: 'Use your preferred shell with a Linux-compatible toolchain. Ensure the executable has permission to run and its directory is on PATH.' }
  ];
</script>

## Toolchain availability

A public download URL and platform installation commands have not been configured for this documentation yet. Use the toolchain supplied by the Vut project maintainers.

> **Important:** There is no verified one-line installer on this page. Avoid running installation scripts from unverified sources.

## Check your installation

Once the `vut` executable is available on your PATH, check its version:

```bash
vut --version
```

Display the available compiler commands:

```bash
vut --help
```

These commands are the same on Windows, macOS, and Linux when a compatible toolchain is installed.

<ContentTabs items={platforms} label="Toolchain platform" />

## Run a program

Save a source file as `hello.vut`, then run:

```bash
vut run hello.vut
```

The compiler builds the program and executes it. Follow [Hello Vut](/docs/getting-started/hello-world/) for a complete example.

## Build a native executable

To compile without running:

```bash
vut build hello.vut
```

For the full command reference, see [Vut CLI](/docs/tooling/vut-cli/).
