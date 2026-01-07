# rs-csv-toolkit

[![CI](https://github.com/philiprehberger/rs-csv-toolkit/actions/workflows/ci.yml/badge.svg)](https://github.com/philiprehberger/rs-csv-toolkit/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/philiprehberger-csv-toolkit.svg)](https://crates.io/crates/philiprehberger-csv-toolkit)
[![Last updated](https://img.shields.io/github/last-commit/philiprehberger/rs-csv-toolkit)](https://github.com/philiprehberger/rs-csv-toolkit/commits/main)

High-level CSV reading, writing, and manipulation

## Installation

```toml
[dependencies]
philiprehberger-csv-toolkit = "0.1.9"
```

## Usage

```rust
use philiprehberger_csv_toolkit::CsvReader;

let data = "name,age,city\nAlice,30,NYC\nBob,25,LA";
let reader = CsvReader::from_str(data);

assert_eq!(reader.headers(), Some(["name", "age", "city"].map(String::from).as_slice()));
assert_eq!(reader.get(0, "name"), Some("Alice"));
assert_eq!(reader.column("age"), Some(vec!["30", "25"]));
```

### Reading from file

```rust
use philiprehberger_csv_toolkit::CsvReader;

let reader = CsvReader::from_path("data.csv").unwrap();
for row in reader.rows() {
    println!("{:?}", row);
}
```

### Writing CSV

```rust
use philiprehberger_csv_toolkit::CsvWriter;

let output = CsvWriter::new()
    .headers(&["name", "score"])
    .row(&["Alice", "95"])
    .row(&["Bob", "87"])
    .to_string();

assert_eq!(output, "name,score\nAlice,95\nBob,87\n");
```

### Custom delimiter

```rust
use philiprehberger_csv_toolkit::CsvReader;

let data = "name;age\nAlice;30";
let reader = CsvReader::from_str(data).delimiter(b';');
assert_eq!(reader.get(0, "name"), Some("Alice"));
```

### Auto-detection

```rust
use philiprehberger_csv_toolkit::CsvReader;

// Delimiter is detected automatically when not explicitly set
let data = "name\tage\nAlice\t30";
let reader = CsvReader::from_str(data);
assert_eq!(reader.get(0, "age"), Some("30"));
```

## API

### `CsvReader`

| Method | Description |
|--------|-------------|
| `from_str(data)` | Parse CSV from a string |
| `from_path(path)` | Read CSV from a file |
| `delimiter(d)` | Set field delimiter (default: auto-detect) |
| `has_headers(b)` | Whether first row is headers (default: true) |
| `headers()` | Get the header row |
| `rows()` | Get all data rows |
| `column(name)` | Get all values in a column by header name |
| `get(row, col)` | Get a single cell by row index and column name |

### `CsvWriter`

| Method | Description |
|--------|-------------|
| `new()` | Create a new writer |
| `delimiter(d)` | Set field delimiter (default: `,`) |
| `headers(h)` | Set header row |
| `row(values)` | Add a data row |
| `to_string()` | Render as CSV string |
| `to_file(path)` | Write CSV to a file |

### `CsvError`

| Variant | Description |
|---------|-------------|
| `IoError(String)` | File I/O error |
| `ParseError { line, message }` | CSV parsing error |

## Development

```bash
cargo test
cargo clippy -- -D warnings
```

## Support

If you find this project useful:

⭐ [Star the repo](https://github.com/philiprehberger/rs-csv-toolkit)

🐛 [Report issues](https://github.com/philiprehberger/rs-csv-toolkit/issues?q=is%3Aissue+is%3Aopen+label%3Abug)

💡 [Suggest features](https://github.com/philiprehberger/rs-csv-toolkit/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement)

❤️ [Sponsor development](https://github.com/sponsors/philiprehberger)

🌐 [All Open Source Projects](https://philiprehberger.com/open-source-packages)

💻 [GitHub Profile](https://github.com/philiprehberger)

🔗 [LinkedIn Profile](https://www.linkedin.com/in/philiprehberger)

## License

[MIT](LICENSE)
