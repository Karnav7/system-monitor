# Publishing to the AUR

`omarchy-system-monitor` isn't on the AUR yet. The recipe in this folder (`PKGBUILD` and `.SRCINFO`) is ready, and it was built and tested with `makepkg` against the v0.1.0 release tarball. It's waiting only on an AUR account, because AUR registrations are temporarily closed.

The name `omarchy-system-monitor` was free on the AUR, in the Arch repos and in omarchy-pkgs when it was chosen. `system-monitor` is already taken on the AUR by an unrelated package.

## One-time setup

1. Create an account at https://aur.archlinux.org/register once registrations reopen, then log in.
2. Under **My Account → SSH Public Key**, paste the public key you push with:

   ```sh
   cat ~/.ssh/id_ed25519.pub
   ```

3. Check that the AUR accepts the key. It should greet you by username rather than say "Permission denied (publickey)":

   ```sh
   ssh aur@aur.archlinux.org help
   ```

4. Optional: add an email to the `# Maintainer:` line in `PKGBUILD`. The AUR convention is `Name <email>`; it currently links to the GitHub profile.

## First upload

The AUR repo must hold only the recipe files at its root, not the whole project.

```sh
git clone ssh://aur@aur.archlinux.org/omarchy-system-monitor.git ~/aur/omarchy-system-monitor
cd ~/aur/omarchy-system-monitor
cp ~/Projects/system-monitor/pkgbuild/PKGBUILD .
makepkg --printsrcinfo > .SRCINFO     # regenerate in case PKGBUILD changed
makepkg -f                            # build and run the tests once more
git add PKGBUILD .SRCINFO
git commit -m "Add omarchy-system-monitor 0.1.0"
git push origin master                # the AUR only accepts the master branch
```

Cloning an unused name gives an empty repo, and the first push creates the package. Don't commit the built `.pkg.tar.zst`, the source tarball or `src/`/`pkg/`.

Check the result at https://aur.archlinux.org/packages/omarchy-system-monitor, then install it the way users will:

```sh
yay -S omarchy-system-monitor
```

On Omarchy it also shows up under the Omarchy menu → **Install → AUR**.

## Releasing a new version

1. Bump `version` in `Cargo.toml`, commit, then tag and push: `git tag v0.2.0 && git push origin main v0.2.0`. The Release workflow builds the binaries and publishes the GitHub release; it fails if the tag doesn't match `Cargo.toml`.
2. In `pkgbuild/PKGBUILD`, set `pkgver=0.2.0` and `pkgrel=1`, then refresh the checksum and metadata:

   ```sh
   cd pkgbuild
   updpkgsums                          # from pacman-contrib
   makepkg --printsrcinfo > .SRCINFO
   makepkg -f
   ```

3. Commit those changes here, then copy `PKGBUILD` and `.SRCINFO` into the AUR clone, commit ("Update to 0.2.0") and push to `master`.

For a packaging-only fix with no new release, bump `pkgrel` instead of `pkgver`.

## After installing the package

A copy installed from source before the package existed is a duplicate. Remove it:

```sh
cargo uninstall system-monitor --root ~/.local
rm ~/.local/share/applications/"System Monitor.desktop"
```

## Later: Omarchy's own package repo

The AUR is the first step. Getting into Omarchy's official repo (omacom/omarchy-pkgs, served from pkgs.omarchy.org) means opening a PR there that adds `pkgbuilds/omarchy-system-monitor/` built from the release tarball, with `.omarchy/package.json` set to `"source": "local"`, `"auto_merge": true` and an `upstream.watch` on this GitHub repo so later releases sync on their own. A maintainer has to approve the build.
