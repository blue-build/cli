# `containerfile`

:::caution
Only BlueBuild builds can use this module as it is a built-in module for the BlueBuild CLI tool.
:::

The `containerfile` module is a tool for adding your own custom instructions for your build that aren't supported by standard `bash` based modules.

Since BlueBuild builds generate a `Contianerfile` from your recipe, you no longer have to manage it yourself. This makes creating your own image easier for less technical users. However, we know that we also have technical users that would like to have the ability to customize their `Containerfile`. This is where the `containerfile` module comes into play. 

## How it works

### `containerfiles`

Below is an example of how a `containerfile` module would be used using the `containerfiles` property:

```yaml
modules:
  - type: containerfile
    containerfiles:
      - example
      - subroutine
```

The `containerfiles` property allows you to tell the compiler which directory contains a `Containerfile` in `./config/containerfiles/`. So in the example above, the compiler would look for these files:

- `./config/containerfiles/example/Containerfile`
- `./config/containerfiles/subroutine/Containerfile`

You could then store files related to say the `subroutine` `Containerfile` in `./config/containerfiles/subroutine/` to keep it organized and portable for other recipes to use.

:::info
**NOTE:** The instructions you add in your `Containerfile`'s each become a layer unlike other modules which are typically ran as a single `RUN` instruction.
:::

### `snippets`

The `snippets` property is the easiest to use when you just need a few lines of instructions without wanting to create a completely new directory and `Containerfile`. Each entry under the `snippets` property will be inserted into your final `Containerfile` for your build.

```yaml
modules:
  - type: contianerfile
    snippets:
      - COPY --from=docker.io/mikefarah/yq /usr/bin/yq /usr/bin/yq
```

This makes it really easy to copy a file or program from another image if it's not available in `rpm-ostree`.

:::info
**NOTE:** Each entry of a snippet will be its own layer in the final `Containerfile`.
:::

### Order of operations

The order you run your instructions is important. So there's a very simple set of rules for the order:

- For each defined module of `containerfile`:
  - All `containerfiles` are printed before any `snippets`
  - All `containerfiles` are printed in the order they are defined
  - All `snippets` are printed in the order they are defined

If you wanted to have some `snippets` run before any `containerfiles` have, you will want to put them in their own module definition before the entry for `containerfiles`. For example:

```yaml
modules:
  - type: contianerfile
    snippets:
      - COPY --from=docker.io/mikefarah/yq /usr/bin/yq /usr/bin/yq
  - type: containerfile
    containerfiles:
      - example
      - subroutine
```

The `COPY` from the `snippets` will always come before the `containerfiles` "example" and "subroutine".
