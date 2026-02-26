## Keeping the binary current

After any changes to `src/`, reinstall before using `hashline` commands:

```sh
cargo install --path .
```

Verify the installed version matches `Cargo.toml` before starting a session that edits source:

```sh
hashline --version
grep '^version' Cargo.toml
```
