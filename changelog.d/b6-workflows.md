For contributors

- CI and the GUI workflow now run on a pushed branch named `ci/<name>`, and CI can be started by hand (`gh workflow run ci.yml --ref ci/<name>`), so a merge result can be checked on real Linux, macOS and Windows before it reaches `main`.
- The release workflow starts with no permissions. The build jobs hold a read-only token and no `GH_TOKEN`; separate publish jobs attest and upload the packages, and the checksums job is the only other writer.
- Every action in every workflow is pinned to a full commit SHA, with its tag in a comment.
