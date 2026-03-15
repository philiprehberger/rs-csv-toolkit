//! High-level CSV reading, writing, and manipulation.
//!
//! Fully RFC 4180 compliant with zero external dependencies. Supports quoted fields,
//! escaped quotes (doubled `""`), newlines within quoted fields, custom delimiters,
//! and automatic delimiter detection.
//!
//! # Reading CSV
//!
//! ```
//! use philiprehberger_csv_toolkit::CsvReader;
//!
//! let data = "name,age,city\nAlice,30,NYC\nBob,25,LA";
//! let reader = CsvReader::parse(data);
//!
//! assert_eq!(reader.get(0, "name"), Some("Alice"));
//! assert_eq!(reader.column("age"), Some(vec!["30", "25"]));
//! ```
//!
//! # Writing CSV
//!
//! ```
//! use philiprehberger_csv_toolkit::CsvWriter;
//!
//! let output = CsvWriter::new()
//!     .headers(&["name", "score"])
//!     .row(&["Alice", "95"])
//!     .row(&["Bob", "87"])
//!     .render();
//!
//! assert_eq!(output, "name,score\nAlice,95\nBob,87\n");
//! ```

use std::fmt;
use std::fs;
use std::io;

/// Errors that can occur during CSV operations.
#[derive(Debug)]
pub enum CsvError {
    /// An I/O error occurred (e.g., file not found).
    IoError(String),
    /// A parsing error occurred at a specific line.
    ParseError {
        /// The 1-based line number where the error occurred.
        line: usize,
        /// A description of the error.
        message: String,
    },
}

impl fmt::Display for CsvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CsvError::IoError(msg) => write!(f, "I/O error: {msg}"),
            CsvError::ParseError { line, message } => {
                write!(f, "parse error at line {line}: {message}")
            }
        }
    }
}

impl std::error::Error for CsvError {}

impl From<io::Error> for CsvError {
    fn from(e: io::Error) -> Self {
        CsvError::IoError(e.to_string())
    }
}

/// Parse RFC 4180 CSV data into rows of fields.
///
/// Handles quoted fields containing delimiters, newlines, and escaped quotes (`""`).
fn parse_csv(input: &str, delimiter: u8) -> Result<Vec<Vec<String>>, CsvError> {
    let delim = delimiter as char;
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();
    let mut logical_line: usize = 1;

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                // Check for escaped quote ""
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    // End of quoted field
                    in_quotes = false;
                }
            } else {
                if c == '\n' {
                    logical_line += 1;
                }
                field.push(c);
            }
        } else if c == '"' {
            if field.is_empty() {
                // Start of quoted field
                in_quotes = true;
            } else {
                // Quote in middle of unquoted field — be lenient, just include it
                field.push(c);
            }
        } else if c == delim {
            current_row.push(std::mem::take(&mut field));
        } else if c == '\n' {
            current_row.push(std::mem::take(&mut field));
            rows.push(std::mem::take(&mut current_row));
            logical_line += 1;
        } else if c == '\r' {
            // Skip \r, handle \r\n
            if chars.peek() == Some(&'\n') {
                // Will be handled by the \n branch on next iteration
            } else {
                // Bare \r acts as line ending
                current_row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut current_row));
                logical_line += 1;
            }
        } else {
            field.push(c);
        }
    }

    if in_quotes {
        return Err(CsvError::ParseError {
            line: logical_line,
            message: "unterminated quoted field".to_string(),
        });
    }

    // Handle last field/row (if file doesn't end with newline)
    if !field.is_empty() || !current_row.is_empty() {
        current_row.push(field);
        rows.push(current_row);
    }

    Ok(rows)
}

/// Detect the most likely delimiter from a set of candidates.
///
/// Tries `,`, `;`, `\t`, and `|`. Picks the delimiter that produces the most
/// consistent (non-zero) column count across all rows.
fn detect_delimiter(input: &str) -> u8 {
    let candidates: &[u8] = b",;\t|";
    let mut best = b',';
    let mut best_score: usize = 0;

    for &delim in candidates {
        if let Ok(rows) = parse_csv(input, delim) {
            if rows.is_empty() {
                continue;
            }
            let first_len = rows[0].len();
            if first_len <= 1 {
                continue;
            }
            // Score = number of rows with the same column count as the first row
            let consistent = rows.iter().filter(|r| r.len() == first_len).count();
            let score = consistent * first_len;
            if score > best_score {
                best_score = score;
                best = delim;
            }
        }
    }

    best
}

/// A CSV reader that parses CSV data and provides access to headers, rows, and cells.
///
/// # Example
///
/// ```
/// use philiprehberger_csv_toolkit::CsvReader;
///
/// let reader = CsvReader::parse("a,b\n1,2\n3,4");
/// assert_eq!(reader.rows().len(), 2);
/// assert_eq!(reader.get(0, "a"), Some("1"));
/// ```
pub struct CsvReader {
    header_row: Option<Vec<String>>,
    data_rows: Vec<Vec<String>>,
    raw: String,
    has_headers: bool,
}

impl CsvReader {
    /// Parse CSV from a string.
    ///
    /// By default, treats the first row as headers and auto-detects the delimiter.
    /// Use [`delimiter`](CsvReader::delimiter) and [`has_headers`](CsvReader::has_headers)
    /// to customize behavior.
    pub fn parse(data: &str) -> Self {
        let delim = detect_delimiter(data);
        let rows = parse_csv(data, delim).unwrap_or_default();
        let mut reader = Self::build(rows, true);
        reader.raw = data.to_string();
        reader
    }

    /// Read CSV from a file path.
    ///
    /// Returns a [`CsvError::IoError`] if the file cannot be read.
    pub fn from_path(path: &str) -> Result<Self, CsvError> {
        let data = fs::read_to_string(path)?;
        let delim = detect_delimiter(&data);
        let rows = parse_csv(&data, delim)?;
        let mut reader = Self::build(rows, true);
        reader.raw = data;
        Ok(reader)
    }

    /// Set the field delimiter, re-parsing the data.
    ///
    /// This replaces auto-detection with the specified delimiter.
    #[must_use]
    pub fn delimiter(self, d: u8) -> Self {
        let rows = parse_csv(&self.raw, d).unwrap_or_default();
        let mut reader = Self::build(rows, self.has_headers);
        reader.raw = self.raw;
        reader
    }

    /// Set whether the first row should be treated as headers.
    ///
    /// When `true` (default), the first row is accessible via [`headers()`](CsvReader::headers)
    /// and is excluded from [`rows()`](CsvReader::rows).
    #[must_use]
    pub fn has_headers(self, b: bool) -> Self {
        let all = self.combined_raw();
        let mut reader = Self::build(all, b);
        reader.raw = self.raw;
        reader
    }

    /// Get the header row, if headers are enabled.
    pub fn headers(&self) -> Option<&[String]> {
        self.header_row.as_deref()
    }

    /// Get all data rows (excluding the header row when headers are enabled).
    pub fn rows(&self) -> &[Vec<String>] {
        &self.data_rows
    }

    /// Get all values for a column by header name.
    ///
    /// Returns `None` if headers are not enabled or the column name is not found.
    pub fn column(&self, name: &str) -> Option<Vec<&str>> {
        let idx = self.col_index(name)?;
        Some(
            self.data_rows
                .iter()
                .filter_map(|row| row.get(idx).map(|s| s.as_str()))
                .collect(),
        )
    }

    /// Get a single cell value by row index and column name.
    ///
    /// Returns `None` if the row index is out of bounds, headers are not enabled,
    /// or the column name is not found.
    pub fn get(&self, row: usize, col: &str) -> Option<&str> {
        let idx = self.col_index(col)?;
        self.data_rows.get(row)?.get(idx).map(|s| s.as_str())
    }

    fn col_index(&self, name: &str) -> Option<usize> {
        self.header_row
            .as_ref()?
            .iter()
            .position(|h| h == name)
    }

    fn build(mut rows: Vec<Vec<String>>, has_headers: bool) -> Self {
        if has_headers && !rows.is_empty() {
            let header_row = rows.remove(0);
            Self {
                header_row: Some(header_row),
                data_rows: rows,
                raw: String::new(),
                has_headers,
            }
        } else {
            Self {
                header_row: None,
                data_rows: rows,
                raw: String::new(),
                has_headers,
            }
        }
    }

    fn combined_raw(&self) -> Vec<Vec<String>> {
        let mut all = Vec::new();
        if let Some(h) = &self.header_row {
            all.push(h.clone());
        }
        all.extend(self.data_rows.clone());
        all
    }
}


/// A CSV writer that builds CSV output from headers and rows.
///
/// # Example
///
/// ```
/// use philiprehberger_csv_toolkit::CsvWriter;
///
/// let csv = CsvWriter::new()
///     .headers(&["x", "y"])
///     .row(&["1", "2"])
///     .render();
///
/// assert_eq!(csv, "x,y\n1,2\n");
/// ```
pub struct CsvWriter {
    delim: u8,
    header_row: Option<Vec<String>>,
    data_rows: Vec<Vec<String>>,
}

impl CsvWriter {
    /// Create a new CSV writer with the default comma delimiter.
    pub fn new() -> Self {
        Self {
            delim: b',',
            header_row: None,
            data_rows: Vec::new(),
        }
    }

    /// Set the field delimiter (default: `,`).
    #[must_use]
    pub fn delimiter(mut self, d: u8) -> Self {
        self.delim = d;
        self
    }

    /// Set the header row.
    #[must_use]
    pub fn headers(mut self, headers: &[&str]) -> Self {
        self.header_row = Some(headers.iter().map(|s| s.to_string()).collect());
        self
    }

    /// Add a data row.
    #[must_use]
    pub fn row(mut self, values: &[&str]) -> Self {
        self.data_rows.push(values.iter().map(|s| s.to_string()).collect());
        self
    }

    /// Render the CSV data as a string.
    pub fn render(&self) -> String {
        let delim_char = self.delim as char;
        let mut out = String::new();

        if let Some(h) = &self.header_row {
            self.write_row(&mut out, h, delim_char);
        }

        for row in &self.data_rows {
            self.write_row(&mut out, row, delim_char);
        }

        out
    }

    /// Write the CSV data to a file.
    pub fn to_file(&self, path: &str) -> Result<(), CsvError> {
        let content = self.render();
        fs::write(path, &content)?;
        Ok(())
    }

    fn write_row(&self, out: &mut String, row: &[String], delim_char: char) {
        for (i, field) in row.iter().enumerate() {
            if i > 0 {
                out.push(delim_char);
            }
            self.write_field(out, field, delim_char);
        }
        out.push('\n');
    }

    fn write_field(&self, out: &mut String, field: &str, delim_char: char) {
        let needs_quoting = field.contains(delim_char)
            || field.contains('"')
            || field.contains('\n')
            || field.contains('\r');

        if needs_quoting {
            out.push('"');
            for c in field.chars() {
                if c == '"' {
                    out.push_str("\"\"");
                } else {
                    out.push(c);
                }
            }
            out.push('"');
        } else {
            out.push_str(field);
        }
    }
}

impl Default for CsvWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_csv() {
        let reader = CsvReader::parse("a,b,c\n1,2,3\n4,5,6");
        assert_eq!(reader.rows().len(), 2);
        assert_eq!(reader.rows()[0], vec!["1", "2", "3"]);
        assert_eq!(reader.rows()[1], vec!["4", "5", "6"]);
    }

    #[test]
    fn parse_with_headers_access_by_column() {
        let reader = CsvReader::parse("name,age\nAlice,30\nBob,25");
        assert_eq!(
            reader.headers(),
            Some(vec!["name".to_string(), "age".to_string()].as_slice())
        );
        assert_eq!(reader.column("name"), Some(vec!["Alice", "Bob"]));
        assert_eq!(reader.column("age"), Some(vec!["30", "25"]));
        assert_eq!(reader.column("missing"), None);
    }

    #[test]
    fn quoted_fields_with_commas() {
        let reader = CsvReader::parse("name,address\nAlice,\"123 Main St, Apt 4\"\nBob,\"456 Oak Ave, Suite 5\"");
        assert_eq!(reader.get(0, "address"), Some("123 Main St, Apt 4"));
        assert_eq!(reader.get(1, "address"), Some("456 Oak Ave, Suite 5"));
    }

    #[test]
    fn quoted_fields_with_embedded_quotes() {
        let reader = CsvReader::parse("name,quote\nAlice,\"She said \"\"hello\"\"\"\nBob,\"He said \"\"bye\"\"\"");
        assert_eq!(reader.get(0, "quote"), Some("She said \"hello\""));
        assert_eq!(reader.get(1, "quote"), Some("He said \"bye\""));
    }

    #[test]
    fn quoted_fields_with_newlines() {
        let data = "name,bio\nAlice,\"Line 1\nLine 2\"\nBob,\"One line\"";
        let reader = CsvReader::parse(data);
        assert_eq!(reader.get(0, "bio"), Some("Line 1\nLine 2"));
        assert_eq!(reader.get(1, "bio"), Some("One line"));
        assert_eq!(reader.rows().len(), 2);
    }

    #[test]
    fn custom_delimiter_semicolon() {
        let data = "name;age\nAlice;30\nBob;25";
        let reader = CsvReader::parse(data).delimiter(b';');
        assert_eq!(reader.get(0, "name"), Some("Alice"));
        assert_eq!(reader.get(1, "age"), Some("25"));
    }

    #[test]
    fn custom_delimiter_tab() {
        let data = "name\tage\nAlice\t30\nBob\t25";
        let reader = CsvReader::parse(data).delimiter(b'\t');
        assert_eq!(reader.get(0, "name"), Some("Alice"));
        assert_eq!(reader.get(1, "age"), Some("25"));
    }

    #[test]
    fn delimiter_auto_detection_semicolon() {
        let data = "name;age;city\nAlice;30;NYC\nBob;25;LA";
        let reader = CsvReader::parse(data);
        assert_eq!(reader.headers().map(|h| h.len()), Some(3));
        assert_eq!(reader.get(0, "name"), Some("Alice"));
        assert_eq!(reader.get(0, "city"), Some("NYC"));
    }

    #[test]
    fn delimiter_auto_detection_tab() {
        let data = "name\tage\tcolor\nAlice\t30\tred\nBob\t25\tblue";
        let reader = CsvReader::parse(data);
        assert_eq!(reader.get(0, "age"), Some("30"));
        assert_eq!(reader.get(1, "color"), Some("blue"));
    }

    #[test]
    fn writer_basic_output() {
        let csv = CsvWriter::new()
            .headers(&["name", "score"])
            .row(&["Alice", "95"])
            .row(&["Bob", "87"])
            .render();
        assert_eq!(csv, "name,score\nAlice,95\nBob,87\n");
    }

    #[test]
    fn writer_quotes_fields_that_need_it() {
        let csv = CsvWriter::new()
            .headers(&["name", "address"])
            .row(&["Alice", "123 Main, Apt 4"])
            .row(&["Bob", "said \"hi\""])
            .render();
        assert_eq!(
            csv,
            "name,address\nAlice,\"123 Main, Apt 4\"\nBob,\"said \"\"hi\"\"\"\n"
        );
    }

    #[test]
    fn writer_quotes_fields_with_newlines() {
        let csv = CsvWriter::new()
            .headers(&["k", "v"])
            .row(&["a", "line1\nline2"])
            .render();
        assert_eq!(csv, "k,v\na,\"line1\nline2\"\n");
    }

    #[test]
    fn round_trip() {
        let original = CsvWriter::new()
            .headers(&["name", "value", "note"])
            .row(&["Alice", "42", "first entry"])
            .row(&["Bob", "99", "has, comma"])
            .row(&["Eve", "0", "said \"hi\""])
            .render();

        let reader = CsvReader::parse(&original);
        assert_eq!(
            reader.headers(),
            Some(
                vec!["name".to_string(), "value".to_string(), "note".to_string()].as_slice()
            )
        );
        assert_eq!(reader.get(0, "name"), Some("Alice"));
        assert_eq!(reader.get(1, "note"), Some("has, comma"));
        assert_eq!(reader.get(2, "note"), Some("said \"hi\""));
    }

    #[test]
    fn empty_fields() {
        let reader = CsvReader::parse("a,b,c\n,,\n1,,3");
        assert_eq!(reader.rows()[0], vec!["", "", ""]);
        assert_eq!(reader.rows()[1], vec!["1", "", "3"]);
    }

    #[test]
    fn get_and_column_accessors() {
        let reader = CsvReader::parse("x,y,z\n1,2,3\n4,5,6\n7,8,9");
        assert_eq!(reader.get(0, "x"), Some("1"));
        assert_eq!(reader.get(2, "z"), Some("9"));
        assert_eq!(reader.get(5, "x"), None);
        assert_eq!(reader.column("y"), Some(vec!["2", "5", "8"]));
    }

    #[test]
    fn has_headers_false() {
        let reader = CsvReader::parse("1,2,3\n4,5,6").has_headers(false);
        assert_eq!(reader.headers(), None);
        assert_eq!(reader.rows().len(), 2);
        assert_eq!(reader.rows()[0], vec!["1", "2", "3"]);
    }

    #[test]
    fn file_read_write() {
        let dir = std::env::temp_dir();
        let path = dir.join("csv_toolkit_test.csv");
        let path_str = path.to_str().unwrap();

        // Write
        CsvWriter::new()
            .headers(&["a", "b"])
            .row(&["1", "2"])
            .row(&["3", "4"])
            .to_file(path_str)
            .unwrap();

        // Read back
        let reader = CsvReader::from_path(path_str).unwrap();
        assert_eq!(reader.headers().map(|h| h.len()), Some(2));
        assert_eq!(reader.get(0, "a"), Some("1"));
        assert_eq!(reader.get(1, "b"), Some("4"));

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn file_read_nonexistent() {
        let result = CsvReader::from_path("/nonexistent/path/file.csv");
        assert!(result.is_err());
    }

    #[test]
    fn writer_custom_delimiter() {
        let csv = CsvWriter::new()
            .delimiter(b';')
            .headers(&["a", "b"])
            .row(&["1", "2"])
            .render();
        assert_eq!(csv, "a;b\n1;2\n");
    }

    #[test]
    fn writer_no_headers() {
        let csv = CsvWriter::new()
            .row(&["1", "2"])
            .row(&["3", "4"])
            .render();
        assert_eq!(csv, "1,2\n3,4\n");
    }

    #[test]
    fn trailing_newline_optional() {
        // With trailing newline
        let r1 = CsvReader::parse("a,b\n1,2\n");
        assert_eq!(r1.rows().len(), 1);

        // Without trailing newline
        let r2 = CsvReader::parse("a,b\n1,2");
        assert_eq!(r2.rows().len(), 1);

        // Both should give same result
        assert_eq!(r1.rows(), r2.rows());
    }

    #[test]
    fn crlf_line_endings() {
        let reader = CsvReader::parse("a,b\r\n1,2\r\n3,4\r\n");
        assert_eq!(reader.rows().len(), 2);
        assert_eq!(reader.get(0, "a"), Some("1"));
        assert_eq!(reader.get(1, "b"), Some("4"));
    }

    #[test]
    fn single_column() {
        let reader = CsvReader::parse("name\nAlice\nBob");
        assert_eq!(reader.column("name"), Some(vec!["Alice", "Bob"]));
    }

    #[test]
    fn unterminated_quote_is_error() {
        let result = parse_csv("a,\"unclosed\n", b',');
        assert!(result.is_err());
    }
}
