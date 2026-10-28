# Corresponding Source

This notice accompanies the object code of **gitgit Desktop** in the `.deb`,
the `.AppImage`, the `.msi` and the `.dmg`. It is committed to the repository
and shipped inside those installers, so it travels with the object code
instead of being generated at build time and drifting away from it.

## The licence

**GNU Affero General Public License, version 3 (AGPL-3.0).**

The full text of the licence is installed in the same directory as this
notice:

- the `.deb`: `usr/share/doc/gitgit-desktop/copyright`
- the `.AppImage`: `usr/share/doc/gitgit-desktop/LICENSE`
- the `.msi`: `LICENSE.rtf`, next to the installed executable
- the `.dmg`: a UDIF EULA resource, presented when the image is mounted

The same text is the `LICENSE` file at the root of the source repository.

## Where the Corresponding Source for this build is offered

The object code in this installer was offered from this project's Releases
page, so the Corresponding Source is offered through the same place, in the
same way, and at no further charge:

- **Designated place — the object code and the Corresponding Source
  alike:** https://github.com/UlyssesLeoLee/GitGit/releases
- **The release that carries this installer** has a "Corresponding Source"
  section in its release notes. That section names the exact source tag this
  build was made from, and the archive URL of that tag's source.
- **The source repository** is at https://github.com/UlyssesLeoLee/GitGit, and
  the source archive of any tag is at
  `https://github.com/UlyssesLeoLee/GitGit/archive/refs/tags/<tag>.tar.gz`.

No account, no registration and no payment is required to obtain the
Corresponding Source from any of the addresses above.

## What this notice does not cover

- The **per-user** `.msi` does not ship this file. The Tauri bundler renders
  every `bundle.resources` entry as a component whose KeyPath is a *file*, and
  ICE38 rejects a component that installs into the user profile whose
  KeyPath is a file — so the per-user WiX template leaves the resources out
  entirely, and the notice with them. The per-machine `.msi` does carry it.
  `docs/plan/v0-tasks.md` records the measured failure this rests on, and
  `docs/plan/agpl-corresponding-source-msi-dmg.md` has the detail.
