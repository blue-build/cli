# `files`

The `files` module simplifies the process of copying files to the image during the build time. These files are sourced from the `config/files` directory, which is located at `/tmp/config/files` inside the image.

:::info
If you want to place any files in `/etc/`, you should place them in `/usr/etc/` instead, which will be used to generate `/etc/` on a booted system. That is the proper directory for "system" configuration templates on atomic Fedora distros, whereas `/etc/` is meant for manual overrides and editing by the machine's admin AFTER installation! See issue https://github.com/ublue-os/startingpoint/issues/28.
:::
