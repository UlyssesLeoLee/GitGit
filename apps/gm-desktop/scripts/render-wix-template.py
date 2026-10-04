#!/usr/bin/env python3
"""Render the per-user WiX template from the per-machine one.

WHY A GENERATOR INSTEAD OF A SECOND HAND-MAINTAINED COPY
=======================================================
`wix/main.wxs` and `wix/main-peruser.wxs` are the same ~500-line WiX
template apart from four edits. Keeping the second file as a manual copy
means the two can silently drift: a Tauri upgrade updates the vendored
template, someone re-diffs the per-machine copy, and the per-user copy is
never noticed until an installer is built from a template whose directory
tree and registry roots no longer agree with the other one.

So `main.wxs` is the single source of truth and this script derives the
per-user variant from it. Every edit is asserted: if an edit's anchor text
is not found, the script fails loudly instead of emitting a file that
looks fine and is wrong. The generated file is committed so that building
never depends on running this first, and so a reviewer can read the diff.

THE SIX EDITS
=============
1. `InstallScope` on `<Package>`: perMachine -> perUser. This is the
   setting that makes msiexec refuse an unelevated install with
   `Error 1925: You do not have sufficient privileges` (return code
   1603), which is what made the installer untestable in a
   non-administrator session.

   Do NOT also add an explicit `<Property Id="ALLUSERS" Value="2"/>`.
   That is the single most misleading part of this whole exercise and
   it was tried here first, from several blog posts that recommend it.
   `ALLUSERS=2` does not mean "per user"; it means "dual purpose, let
   the installer decide at run time based on the user's privileges".
   Adding it on top of `InstallScope="perUser"` overrides the scope WiX
   already wrote, and on a non-elevated session the system then picks
   per-machine - so the package installs as per-user in some respects
   and still fails at `InstallFinalize` with Error 1925 in others. A
   pure per-user package wants `ALLUSERS` to be the empty string, which
   WiX expresses through `InstallScope` and refuses to let you spell as
   a `Property` (`CNDL0006: The Property/@Value attribute's value cannot
   be an empty string`). The absence of an explicit ALLUSERS property
   here is load-bearing; do not "helpfully" add one.

2. The install directory moves from `ProgramFiles[64]Folder` to
   `LocalAppDataFolder\\Programs`. A per-machine package writing to
   Program Files cannot be serviced per-user, and a per-user package
   writing there is both wrong and, on most machines, still privileged.
   `LocalAppDataFolder` is a WiX built-in well-known directory.

3. The deep-link registry root: HKLM -> HKCU. Upstream's template even
   carries the comment "Change the Root to HKCU for perUser
   installations" directly above this block. It is currently inert
   because no deep-link protocol is configured in tauri.conf.json, but
   leaving it as HKLM would fail the first time someone adds one.

4. The `Path` component moves its KeyPath from the installed file to an
   HKCU registry value. This is ICE38, and it is a hard error, not a
   warning: once a component installs into the user profile, Windows
   Installer requires the keypath to be a per-user registry value so
   that the component can be resolved without elevation. Every other
   component in the template already uses an HKCU keypath, so `Path`
   was the only one that had to move.

5. A `RemoveFolder` for `ProgramsFolder`, declared from a component that
   lives *inside* `ProgramsFolder` and is referenced from the MainProgram
   feature. This is ICE64, also a hard error: a directory inside the user
   profile has to be listed for removal. The placement matters more than
   it looks - attaching the same `RemoveFolder` to the uninstall
   component under `INSTALLDIR` does put the directory in the MSI
   RemoveFile table (verified by reading the table back out of a
   built MSI) and ICE64 still fails. `RemoveFolder` only deletes an
   empty directory, so sibling per-user applications are unaffected.

Usage:  python apps/gm-desktop/scripts/render-wix-template.py
"""

from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
WIX_DIR = os.path.normpath(os.path.join(HERE, "..", "src-tauri", "wix"))
SOURCE = os.path.join(WIX_DIR, "main.wxs")
TARGET = os.path.join(WIX_DIR, "main-peruser.wxs")

HEADER = """<?xml version="1.0" encoding="UTF-8"?>
<!--
  GENERATED FILE - DO NOT EDIT BY HAND
  ====================================
  Produced by apps/gm-desktop/scripts/render-wix-template.py from
  wix/main.wxs. Edit the source template and re-run the script; the four
  edits it applies are listed both there and in the script's docstring.

  This is the PER-USER variant: it installs into
  %LOCALAPPDATA%\\Programs without administrator rights. The PER-MACHINE
  variant is main.wxs.

  Build the per-machine package with:
      pnpm tauri build (bundles msi)
  Build the per-user package with:
      pnpm tauri build (bundles msi, config tauri.peruser.conf.json)

  NOTE ON THIS COMMENT: an XML comment may not contain two consecutive
  hyphens anywhere inside it, not even inside what looks like prose or a
  command line. candle reports a misleading CNDL0104 "not a valid source
  file" when it happens, because the comment is terminated early and the
  remainder becomes stray text in front of the root element. Command
  lines above are therefore written without their usual double-hyphen
  flags. Keep it that way when editing.
-->
"""

# (description, source text, replacement text)
EDITS = [
    (
        "InstallScope: perMachine -> perUser",
        '                 InstallScope="perMachine"\n',
        '                 InstallScope="perUser"\n',
    ),
    (
        "install directory: Program Files -> %LOCALAPPDATA%\\Programs",
        '            <Directory Id="$(var.PlatformProgramFilesFolder)" Name="PFiles">\n'
        '                <Directory Id="INSTALLDIR" Name="{{product_name}}"/>\n'
        "            </Directory>\n",
        '            <Directory Id="LocalAppDataFolder" Name="LocalAppDataFolder">\n'
        '                <Directory Id="ProgramsFolder" Name="Programs">\n'
        '                    <Directory Id="INSTALLDIR" Name="{{product_name}}"/>\n'
        "                </Directory>\n"
        "            </Directory>\n",
    ),
    (
        "deep-link registry root: HKLM -> HKCU",
        '                <RegistryKey Root="HKLM" Key="Software\\Classes\\\\{{protocol}}">\n',
        '                <RegistryKey Root="HKCU" Key="Software\\Classes\\\\{{protocol}}">\n',
    ),
    (
        "ICE38: Path component keypath moves from the file to HKCU",
        '            <Component Id="Path" Guid="{{path_component_guid}}" Win64="$(var.Win64)">\n'
        '                <File Id="Path" Source="{{main_binary_path}}" KeyPath="yes" Checksum="yes"/>\n',
        '            <Component Id="Path" Guid="{{path_component_guid}}" Win64="$(var.Win64)">\n'
        "                <!-- ICE38: a component installing into the user profile\n"
        "                     must use an HKCU registry value as its KeyPath. -->\n"
        '                <RegistryValue Root="HKCU" Key="Software\\\\{{manufacturer}}\\\\{{product_name}}"\n'
        '                               Name="Path" Type="string" Value="[INSTALLDIR]" KeyPath="yes" />\n'
        '                <File Id="Path" Source="{{main_binary_path}}" Checksum="yes"/>\n',
    ),
    (
        "ICE64: remove ProgramsFolder from a component inside that directory",
        '        <DirectoryRef Id="ApplicationProgramsFolder">\n',
        '        <DirectoryRef Id="ProgramsFolder">\n'
        "            <!-- ICE64: a directory inside the user profile has to be\n"
        "                 listed for removal. Declaring the RemoveFolder from a\n"
        "                 component that LIVES in ProgramsFolder is what satisfies\n"
        "                 the check; the same RemoveFolder attached to the\n"
        "                 uninstall component under INSTALLDIR puts the directory\n"
        "                 in the RemoveFile table and still fails. RemoveFolder\n"
        "                 only deletes an empty directory, so sibling per-user\n"
        "                 applications are unaffected. -->\n"
        '            <Component Id="CMP_ProgramsFolderCleanup" Guid="*">\n'
        '                <RegistryValue Root="HKCU" Key="Software\\\\{{manufacturer}}\\\\{{product_name}}"\n'
        '                               Name="ProgramsFolderCleanup" Type="integer" Value="1" KeyPath="yes" />\n'
        '                <RemoveFolder Id="ProgramsFolder" On="uninstall" />\n'
        "            </Component>\n"
        "        </DirectoryRef>\n"
        "\n"
        '        <DirectoryRef Id="ApplicationProgramsFolder">\n',
    ),
    (
        "reference the cleanup component from the MainProgram feature",
        '            <ComponentRef Id="RegistryEntries"/>\n',
        '            <ComponentRef Id="RegistryEntries"/>\n'
        '            <ComponentRef Id="CMP_ProgramsFolderCleanup" />\n',
    ),
]


def main() -> int:
    if not os.path.exists(SOURCE):
        print(f"error: source template not found: {SOURCE}", file=sys.stderr)
        return 1

    with open(SOURCE, encoding="utf-8") as fh:
        text = fh.read()

    # Drop the source file's own header; the generated file gets its own.
    marker = '<?if $(sys.BUILDARCH)="x86"?>'
    idx = text.find(marker)
    if idx == -1:
        print("error: could not find the buildarch block in the source template", file=sys.stderr)
        return 1
    body = text[idx:]

    applied = 0
    for desc, src, dst in EDITS:
        count = body.count(src)
        if count != 1:
            print(
                f"error: edit anchor matched {count} times, expected exactly 1: {desc}\n"
                f"       anchor: {src!r}",
                file=sys.stderr,
            )
            return 1
        body = body.replace(src, dst)
        applied += 1

    with open(TARGET, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(HEADER + body)

    print(f"wrote {TARGET} ({os.path.getsize(TARGET)} bytes, {applied} edits applied)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
