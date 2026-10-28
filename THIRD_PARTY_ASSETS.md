# Third-Party and User-Supplied Assets

The PolyForm Noncommercial license in `LICENSE` covers honk300's project-authored source code, documentation, and project-authored media. It does not relicense assets owned by other people.

## Personal-use compatibility assets

The following directories contain screened compatibility media retained at the project owner's direction for personal-use builds:

- `Assets/Sounds/`
- `Assets/Images/Memes/originals/`
- `Assets/Text/NotepadMessages/originals/`
- `Assets/Toys/` and its panel copies in `relay/public/toys/`

`Assets/Toys/ball*.png` and `stick*.png` come from the community ToysMod v1.1 for Desktop Goose,
and `Assets/Toys/crumbs.png` from the community BreadCrumbs mod, both as shipped in the operator's
Desktop Goose v0.31 folder. Only these images are used. No mod code, DLLs or sounds are included.

Those files remain copyright their respective owners. honk300 does not assert ownership of them or grant redistribution, commercial-use, trademark, publicity, or other rights beyond rights the recipient already has. Anyone redistributing a build is responsible for confirming permission or replacing/removing these files.

The screening record in `Assets/Images/Memes/originals/SCREENING.md` is part of this notice. Material containing old donation links, Patreon references, social handles, or old-developer promotion is intentionally excluded.

## Project-authored counterparts

Media under these directories was created specifically for honk300 and is covered by the project's license unless a file says otherwise:

- `Assets/Images/Memes/custom/`
- `Assets/Text/NotepadMessages/custom/`

Generation prompts and provenance notes are kept beside the custom meme assets.

## User-supplied media

`Assets/Images/Memes/user/Meme8.png` was supplied and approved by the project owner. Its underlying rights were not independently audited; downstream distributors must make their own permission determination.

Runtime user media stored in a user's honk300 data directory remains that user's content and is never relicensed by honk300.

## Dependencies

Rust crates and build tools retain their own licenses. Their inclusion does not change those terms. Release archives must include this notice and the root `LICENSE` file.

### Vendored Wayland scanner backport

`vendor/wayland-scanner/` contains the MIT-licensed crates.io `wayland-scanner` 0.31.10 source.
Its included `LICENSE.txt` remains authoritative. honk300 updates only its `quick-xml` dependency
and corresponding parser call to carry the security fix from upstream wayland-rs revision
`d07c4f91f28b42e5a485823ffd9d8d5a210b1053` without adopting that revision's unreleased breaking
Wayland API changes. Full provenance and removal criteria are in
`vendor/wayland-scanner/UPSTREAM.md`.
