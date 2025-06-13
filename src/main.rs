use clap::Parser;
use csv::Writer;
use std::error::Error;
use std::path::PathBuf;
use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn parse_opt_f64(s: &str) -> Option<f64> {
    if s.trim().is_empty() {
        None
    } else {
        s.trim().parse::<f64>().ok()
    }
}

fn detect_series_name(fields: &[&str]) -> Option<String> {
    if fields.len() == 2 && fields[0].starts_with("Series Name") {
        Some(fields[1].into())
    } else {
        None
    }
}
/// Detects the protocol name from the CSV header
/// Note that Protocol Name can occur in different contexts. This should only be
/// called if in the InSeries state.
fn detect_protocol_name(fields: &[&str]) -> Option<String> {
    if fields.len() == 2 && fields[0].starts_with("Protocol Name") && fields[1] != "" {
        eprintln!("protocol name {}", fields[1]);
        Some(fields[1].into())
    } else {
        None
    }
}

/// Checks if the current line is a measurement header.
/// The Measurement headers must start with Measurement, have 8 fields and end with
/// Instance 2.
fn is_measurement_header(fields: &[String]) -> bool {
    fields.len() == 8 && fields[0].starts_with("Measurement") && fields[7].starts_with("Instance 2")
}

/// Checks if the current line is a calculation start.
/// The Calculation headers must start with Calculation, have length 4 where the third
/// column is "Units".
fn is_calculation_start(fields: &[String]) -> bool {
    fields.len() == 4 && fields[0].starts_with("Calculation") && fields[2].starts_with("Units")
}

/// Represents the current parsing state of the CSV parser.
///
/// This enum drives the finite state machine used to interpret lines
/// in a VisualSonics Vevo LAB csv file. It helps manage transitions between
/// different sections of the file, such as series, measurements, and calculations.
#[derive(Debug)]
enum ParseState {
    Idle,
    InSeries { id: String },
    InProtocol { id: String, protocol: String },
    InMeasurement { id: String, protocol: String },
    InCalculation { id: String, protocol: String },
}

/// Trait for CSV-parsable rows that include an `id`, `protocol`, and custom fields.
pub trait CsvRow: Sized {
    /// Construct from a Series ID, Protocol name, and a slice of field strings.
    fn new(id: String, protocol: String, fields: &[&str]) -> Self;

    /// Return the field names in CSV serialization order.
    fn return_fields() -> Vec<String>;
}

/// A single measurement row parsed from the "Measurement" table of a VisualSonics CSV export.
///
/// Each row corresponds to one measurement for a given protocol and series.
///
/// # Example
/// A line like:
/// ```csv
/// "A'","PW Tissue Doppler Mode","Velocity","mm/s","-14.438560","0.000000","-14.438560",,
/// ```
/// Would be parsed into:
/// - `measurement`: Some("A'")
/// - `mode`: Some("PW Tissue Doppler Mode")
/// - `parameter`: Some("Velocity")
/// - `units`: Some("mm/s")
/// - `avg`: Some(-14.438560)
/// - etc.
///
/// The `id` and `protocol` fields are added based on
/// the current context (Series Name and Protocol Name).
#[derive(serde::Serialize)]
pub struct MeasurementRow {
    /// The Series Name (identifier for the sample)
    id: String,
    /// The Protocol Name (identifier for the measurement type)
    protocol: String,
    measurement: Option<String>,
    mode: Option<String>,
    parameter: Option<String>,
    units: Option<String>,
    avg: Option<f64>,
    std: Option<f64>,
    instance_1: Option<f64>,
    instance_2: Option<f64>,
}

impl CsvRow for MeasurementRow {
    fn new(id: String, protocol: String, fields: &[&str]) -> Self {
        Self {
            id,
            protocol,
            measurement: fields.get(0).map(|s| s.trim_matches('"').to_string()),
            mode: fields.get(1).map(|s| s.trim_matches('"').to_string()),
            parameter: fields.get(2).map(|s| s.trim_matches('"').to_string()),
            units: fields.get(3).map(|s| s.trim_matches('"').to_string()),
            avg: fields.get(4).and_then(|s| parse_opt_f64(s)),
            std: fields.get(5).and_then(|s| parse_opt_f64(s)),
            instance_1: fields.get(6).and_then(|s| parse_opt_f64(s)),
            instance_2: fields.get(7).and_then(|s| parse_opt_f64(s)),
        }
    }

    fn return_fields() -> Vec<String> {
        vec![
            "id".to_string(),
            "protocol".to_string(),
            "measurement".to_string(),
            "mode".to_string(),
            "parameter".to_string(),
            "units".to_string(),
            "avg".to_string(),
            "std".to_string(),
            "instance_1".to_string(),
            "instance_2".to_string(),
        ]
    }
}

#[derive(serde::Serialize)]
struct CalculationRow {
    id: String,
    protocol: String,
    calculation: Option<String>, // e.g., "EF"
    units: Option<String>,       // e.g., "%"
    value: Option<f64>,
}

impl CsvRow for CalculationRow {
    /// This takes a Series ID, Protocol name, and a slice of field strings which is
    /// parsed directly from the CSV. the Calculation table structure has 4 columns,
    /// but the second is empty, hence using index 0, 2 and 3
    fn new(id: String, protocol: String, fields: &[&str]) -> Self {
        Self {
            id,
            protocol,
            calculation: fields.get(0).map(|s| s.trim_matches('"').to_string()),
            units: fields.get(2).map(|s| s.trim_matches('"').to_string()),
            value: fields.get(3).and_then(|s| parse_opt_f64(s)),
        }
    }

    fn return_fields() -> Vec<String> {
        vec![
            "id".to_string(),
            "protocol".to_string(),
            "calculation".to_string(),
            "units".to_string(),
            "value".to_string(),
        ]
    }
}

pub trait CsvTable<T>: Sized {
    fn new() -> Self;
    fn add_row(&mut self, row: T);
    fn write_csv(&self, path: &str) -> Result<(), Box<dyn Error>>;
}

struct MeasurementTable {
    rows: Vec<MeasurementRow>,
}

impl CsvTable<MeasurementRow> for MeasurementTable {
    fn new() -> Self {
        MeasurementTable { rows: Vec::new() }
    }

    fn add_row(&mut self, row: MeasurementRow) {
        self.rows.push(row);
    }

    fn write_csv(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let mut wtr = Writer::from_path(path)?;
        for row in &self.rows {
            wtr.serialize(row)?;
        }
        wtr.flush()?;
        Ok(())
    }
}

struct CalculationTable {
    rows: Vec<CalculationRow>,
}

impl CsvTable<CalculationRow> for CalculationTable {
    fn new() -> Self {
        CalculationTable { rows: Vec::new() }
    }

    fn add_row(&mut self, row: CalculationRow) {
        self.rows.push(row);
    }

    fn write_csv(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let mut wtr = Writer::from_path(path)?;
        for row in &self.rows {
            wtr.serialize(row)?;
        }
        wtr.flush()?;
        Ok(())
    }
}

fn parse_vevolab_csv(
    input_path: &Path,
) -> Result<(MeasurementTable, CalculationTable), Box<dyn Error>> {
    let file = File::open(input_path)?;
    let reader = BufReader::new(file);
    let mut state = ParseState::Idle;

    let mut measurement_table = MeasurementTable::new();
    let mut calculation_table = CalculationTable::new();

    for line_result in reader.lines() {
        let line = line_result?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }


        let record = line.split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<String>>();
 
        // Skip empty lines
        if record.iter().all(|s| s.trim().is_empty()) {
            continue;
        }
        let fields: Vec<String> = record
            .iter()
            .map(|s| s.trim().trim_matches('"').to_string())
            .collect();
        let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();

        match &state {
            ParseState::Idle => {
                if let Some(series_id) = detect_series_name(&field_refs) {
                    state = ParseState::InSeries { id: series_id };
                }
            }
            ParseState::InSeries { id } => {
                if let Some(protocol_name) = detect_protocol_name(&field_refs) {
                    state = ParseState::InProtocol {
                        id: id.clone(),
                        protocol: protocol_name,
                    };
                }
            }
            ParseState::InProtocol { id, protocol } => {
                if is_measurement_header(&fields) {
                    state = ParseState::InMeasurement {
                        id: id.clone(),
                        protocol: protocol.clone(),
                    };
                }
            }
            ParseState::InMeasurement { id, protocol } => {

                if is_calculation_start(&fields){
                    state = ParseState::InCalculation {
                        id: id.clone(),
                        protocol: protocol.clone(),
                    };
                    continue; // Skip to next iteration to handle calculation
                } else{

                    let row = MeasurementRow::new(id.clone(), protocol.clone(), &field_refs);
                    measurement_table.add_row(row);
                }
            }
            ParseState::InCalculation { id, protocol } => {
                if let Some(protocol_name) = detect_protocol_name(&field_refs) {
                    state = ParseState::InProtocol {
                        id: id.clone(),
                        protocol: protocol_name,
                    };
                    continue;
                }
                if let Some(series_id) = detect_series_name(&field_refs) {
                    state = ParseState::InSeries { id: series_id };
                } else {
                    let row = CalculationRow::new(id.clone(), protocol.clone(), &field_refs);
                    calculation_table.add_row(row);
                }
            }
        }

        // print state
        // eprintln!("State: {:?}", state);
    }

    Ok((measurement_table, calculation_table))
}

/// Extract measurement and calculation data from a VisualSonics CSV export.
///
/// This tool parses a `.csv` file and extracts structured tables for downstream analysis.
/// It checks for protocol sections, measurement blocks, and calculation rows.
///
/// # Example
/// ```bash
/// vevolabparser data/input.csv
/// ```
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Path to the input CSV file
    #[arg(value_name = "CSV_FILE", value_hint = clap::ValueHint::FilePath)]
    input: PathBuf,
}


fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if !cli.input.is_file() {
        eprintln!(
            "Error: '{}' does not exist or is not a regular file.",
            cli.input.display()
        );
        std::process::exit(1);
    }

    eprintln!("Parsing file: {}", cli.input.display());

    let (measurement_table, calculation_table) = parse_vevolab_csv(&cli.input)?;

    let output_prefix = cli
        .input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("input");

    let measurement_path = format!("{}_measurements.csv", output_prefix);
    let calculation_path = format!("{}_calculations.csv", output_prefix);

    measurement_table.write_csv(&measurement_path)?;
    calculation_table.write_csv(&calculation_path)?;

    eprintln!(
        "Parsing complete. Output written to {} and {}.",
        measurement_path, calculation_path
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*; // gives access to private items in main.rs

    #[test]
    fn test_detect_series_name() {
        let fields = vec![
            "Series Name",
            "10-a",
        ];
        let result = detect_series_name(&fields);
        assert_eq!(result, Some("10-a".to_string()));
    }

    #[test]
    fn test_detect_protocol_name() {
        let fields = vec![
            "Protocol Name",
            "MV Flow",
        ];
        let result = detect_protocol_name(&fields);
        assert_eq!(result, Some("MV Flow".to_string()));
    }

    #[test]
    fn test_measurement_row_parsing() {
        let fields = vec![
            "A'",
            "PW Tissue Doppler Mode",
            "Velocity",
            "mm/s",
            "-14.438560",
            "0.000000",
            "42.0",
            "",
        ];
        let row = MeasurementRow::new("12-0".into(), "MV Flow".into(), &fields);

        assert_eq!(row.measurement.as_deref(), Some("A'"));
        assert_eq!(row.mode.as_deref(), Some("PW Tissue Doppler Mode"));
        assert_eq!(row.avg, Some(-14.43856));
        assert_eq!(row.instance_2, None);
    }

    #[test]
    fn test_calculation_row_parsing() {
        let fields = vec!["A'/E'", "", "none", "1.538462"];
        let row = CalculationRow::new("12-0".into(), "SAX M-Mode".into(), &fields);

        assert_eq!(row.calculation.as_deref(), Some("A'/E'"));
        assert_eq!(row.units.as_deref(), Some("none"));
        assert_eq!(row.value, Some(1.538462));
    }

    #[test]
    fn test_is_calculation_start() {
        let fields = vec![
            "Calculation".to_string(),
            "".to_string(),
            "Units".to_string(),
            "".to_string(),
        ];
        assert!(is_calculation_start(&fields));
    }

    #[test]
    fn test_measurement_table_add_row() {
        let fields = vec![
            "A'",
            "PW Tissue Doppler Mode",
            "Velocity",
            "mm/s",
            "-14.438560",
            "0.000000",
            "42.0",
            "",
        ];
        let row = MeasurementRow::new("12-0".into(), "MV Flow".into(), &fields);

        let mut table = MeasurementTable::new();
        table.add_row(row);

        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].measurement.as_deref(), Some("A'"));
    }

    #[test]
    fn test_calculation_table_add_row() {
        let fields = vec!["A'/E'", "", "none", "1.538462"];
        let row = CalculationRow::new("12-0".into(), "SAX M-Mode".into(), &fields);

        let mut table = CalculationTable::new();
        table.add_row(row);

        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].value, Some(1.538462));
    }
}
