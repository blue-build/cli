# `copy`

:::caution
Only compiler-based builds can use this module as it is built-in to the BlueBuild CLI tool.
:::

:::note
**NOTE:** This module is currently only available with the `use_unstable_cli` option on the GHA or using the `main` image.
:::

The `copy` module is a short-hand method of adding a [`COPY`]() instruction into the image. This can be used to copy files from images, other stages, or even from the build context. 

## Usage

The `copy` module's properties are a 1-1 match with the `COPY` instruction containing `src`, `dest`, and `from` (optional). The example below will `COPY` the file `/usr/bin/yq` from `docker.io/mikefarah/yq` into `/usr/bin/`.

```yaml
mdoules:
- type: copy
  from: docker.io/mikefarah/yq
  src: /usr/bin/yq
  dest: /usr/bin/
```

Creating an instruction like:

```dockerfile
COPY --linked --from=docker.io/mikefarah/yq /usr/bin/yq /usr/bin/
```

Omitting `from:` will allow copying from the build context:

```yaml
mdoules:
- type: copy
  src: file/to/copy.conf
  dest: /usr/etc/app/
```

Creating an instruction like:

```dockerfile
COPY --linked file/to/copy.conf /usr/etc/app/
```
