# Templates

This directory contains **workspace templates** used by `envoyctl init`.

When a user runs:

envoyctl init --dir <path>

the contents of `templates/workspace/` are copied into the target directory.

These templates provide:
- a valid starting Envoy configuration
- documented defaults
- example domains, upstreams, and policies

⚠️ Do not put runtime code here.  
⚠️ Everything in this directory must be safe to copy verbatim.
