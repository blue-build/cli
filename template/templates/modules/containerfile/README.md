# `containerfile`

:::caution
Only compiler-based builds can use this module as it is built-in to the BlueBuild CLI tool.
:::

The `containerfile` module is a tool for adding custom [`Containerfile`](https://github.com/containers/common/blob/main/docs/Containerfile.5.md) instructions for custom image builds. This is useful when you wish to use some feature directly available in a `Containerfile`, but not in a bash module, such as copying from other OCI images with `COPY --from`.

Since standard compiler-based BlueBuild image builds generate a `Containerfile` from your recipe, there is no need to manage it yourself. However, we know that we also have technical users that would like to have the ability to customize their `Containerfile`. This is where the `containerfile` module comes into play. 

## Usage

### `snippets:`

The `snippets` property is the easiest to use when you just need to insert a few custom lines to the `Containerfile`. Each entry under the `snippets` property will be directly inserted into your final `Containerfile` for your build.

```yaml
modules:
  - type: containerfile
    snippets:
      - COPY --from=docker.io/mikefarah/yq /usr/bin/yq /usr/bin/yq
```

This makes it really easy to copy a file or program from another image.

:::note
**NOTE:** Each entry of a snippet will be its own layer in the final `Containerfile`.
:::

### `containerfiles:`

The `containerfiles` property allows you to tell the compiler which directory contains a `Containerfile` in `./containerfiles/`. 

Below is an example of how a `containerfile` module would be used with the `containerfiles` property:

```yaml
modules:
  - type: containerfile
    containerfiles:
      - example
      - subroutine
```

In the example above, the compiler would look for these files:

- `./containerfiles/example/Containerfile`
- `./containerfiles/subroutine/Containerfile`

You could then store files related to say the `subroutine` `Containerfile` in `./containerfiles/subroutine/` to keep it organized and portable for other recipes to use.

:::note
**NOTE:** The instructions you add in your `Containerfile`'s each become a layer unlike other modules which are typically run as a single `RUN` command, thus creating only one layer.
:::

### Order of operations

The order of operations is important in a `Containerfile`. There's a very simple set of rules for the order in this module:

- For each defined `containerfile` module:
  - First all `containerfiles:` are added to the main `Containerfile` in the order they are defined
  - Then all `snippets` are added to the main `Containerfile` in the order they are defined

If you wanted to have some `snippets` run before any `containerfiles` have, you will want to put them in their own module definition before the entry for `containerfiles`. For example:

```yaml
modules:
  - type: containerfile
    snippets:
      - COPY --from=docker.io/mikefarah/yq /usr/bin/yq /usr/bin/yq
  - type: containerfile
    containerfiles:
      - example
      - subroutine
```

In the example above, the `COPY` from the `snippets` will always come before the `containerfiles` "example" and "subroutine".
