# Security policy

## Reporting a vulnerability

Please **don't open a public issue** for security problems. Report them privately through GitHub instead:
**[Report a vulnerability](https://github.com/mrzkprtm/qclipz-desktop/security/advisories/new)**.

Include what you found, how to reproduce it, and which version you tested. You'll get a reply as soon as possible, and credit in the release notes if you'd like it.

## Supported versions

QClipz is in pre-release (0.x). Security fixes go into the latest release only.

## Scope notes

- QClipz makes no network connections; it records locally and writes files only to the folder you choose (plus temporary files next to it while recording).
- The UI can only open folders and `.mp4` files, and only recordings you made can be deleted (to the Recycle Bin).
- The bundled FFmpeg is a third-party binary (see `THIRD_PARTY_NOTICES.md`). Vulnerabilities in FFmpeg itself should also be reported upstream.
