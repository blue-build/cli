# `files`

The `files` module simplifies the process of copying files to the image during the build time. These files are sourced from the `config/files` directory, which is located at `/tmp/config/files` inside the image.

:::note
If you want to place any files in `/etc/`, you should place them in `/usr/etc/` instead, which will be used to generate `/etc/` on a booted system. That is the proper directory for "system" configuration templates on atomic Fedora distros, whereas `/etc/` is meant for manual overrides and editing by the machine's admin AFTER installation! See issue https://github.com/blue-build/legacy-template/issues/28.
:::

## Implementation differences between the legacy template and compiler-based builds

When using a compiler-based build (which is the recommended option for all users, so if you don't know what you're using you're probably using that), each instruction under `files:` creates its on layer in the final image using the `Containerfile` `COPY`-command. This module is entirely part of the recipe compiler.

When using a legacy template, all modules are combined into one layer in the final image. With a repo based on the legacy template, the bash version is used. 

The API for both of these options remains exactly the same.