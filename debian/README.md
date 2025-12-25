# Debian packaging

This directory contains Debian packaging metadata.

The package is built using a rules-based (debhelper) approach.

## Key files

- control     Package metadata and dependencies
- rules       Build and install logic
- changelog   Package version history
- postinst    Post-install actions
- prerm       Pre-removal actions
- postrm      Post-removal actions

Systemd units and default configuration files are also shipped from here.

This directory is only relevant when building `.deb` packages.
