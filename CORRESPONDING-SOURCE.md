# Corresponding Source

This notice accompanies the object code of **gitgit Desktop** in the `.deb` and
the `.AppImage`. It is committed to the repository and shipped inside those
two installers, so it travels with the object code instead of being generated
at build time and drifting away from it.

## The licence

**GNU Affero General Public License, version 3 (AGPL-3.0).**

The full text of the licence is installed in the same directory as this
notice:

- the `.deb`: `usr/share/doc/gitgit-desktop/copyright`
- the `.AppImage`: `usr/share/doc/gitgit-desktop/LICENSE`

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

- The `.msi` installer ships the licence as `LICENSE.rtf` next to the
  installed executable, but it does not ship this file. The WiX template this
  repository builds has one slot for a text file and that slot carries the
  licence; see `docs/plan/v0-tasks.md` for the recorded reason.
- The `.dmg` carries the licence as a UDIF EULA resource that is presented
  when the disk image is mounted. There is no file inside the image to attach
  this notice to, and no configuration key that puts one there.
