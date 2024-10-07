# Changelog

## 0.1.5 (2026-03-16)

- Add README badges
- Synchronize version across Cargo.toml, README, and CHANGELOG

## 0.1.0 (2026-03-15)

- Initial release
- RFC 4180 compliant CSV parsing (quoted fields, escaped quotes, newlines in fields)
- `CsvReader` with header access, column/cell lookups, and delimiter auto-detection
- `CsvWriter` with automatic field quoting
- File and string I/O
- Zero dependencies
