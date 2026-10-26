# Contributing

Issues and pull requests are welcome in English or Chinese.

1. Read [V1](docs/PROTOCOL-V1.md). Preserve palette and bit constants; wire changes need a new version and migration proposal.
2. Run Rust tests, formatting, clippy, `npm run build` and `npm test`. Generate vectors with the independent reference encoder; do not edit expected values to match a regression.
3. Scanner fixes should include reproducible images or deterministic synthetic fixtures and a rejection case. Use only consented images without private backgrounds or identifying details.
4. Separate executed evidence from pending physical tests. Tie performance claims to hardware, browser, sample size and measurement method.

Do not weaken CRC, ambiguity checks or camera confirmation to improve apparent success. Visual changes must remain reversible. Samples must come from the actual encoder. Contributions are under the repository's MIT license.
