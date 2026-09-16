---
title: Hello Vut
description: 'Write, run, and understand your first Vut program.'
section: Getting started
---

<script>
  import ContentTabs from '$lib/components/docs/ContentTabs.svelte';
  const example = [
    { label: 'Command', code: 'vut run hello.vut', text: 'Run this command with the local toolchain.' },
    { label: 'Expected output', code: 'Hello, World!', text: 'This is the expected output of the source above, not a program executed by this website.' }
  ];
</script>

## Write your first program

Create a file named `hello.vut`:

```vut title="hello.vut"
fn main():
  name: str = "World"
  out("Hello, $name!")
```

The `fn` keyword declares a function. The colon starts its body, and the two-space indentation marks the statements inside.

## Run it

With the toolchain installed, run the source file:

```bash
vut run hello.vut
```

The program prints the greeting `Hello, World!`.

<ContentTabs items={example} label="Hello Vut example" />

## Make it yours

Change the value of `name` to your own name. Within a string, `$name` inserts the value of that variable.

You can also embed an expression with `$(expression)`:

```vut
fn main():
  out("Two plus three is $(2 + 3)")
```

## Build without running

```bash
vut build hello.vut
```

Next, learn how files fit together in a [Vut project](/docs/getting-started/project-structure/).
