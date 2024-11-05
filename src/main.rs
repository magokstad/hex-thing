use std::{
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Read, Seek, Write},
    path::PathBuf,
    str::FromStr,
};

use clap::Parser;
use colored::{Color, Colorize};
use lazy_static::lazy_static;

#[derive(Debug, Clone, Default)]
struct ByteRange {
    pub start: usize,
    pub end: usize,
}

impl FromStr for ByteRange {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 2 {
            return Err("Range must be in the format 'start-end'");
        }

        let start = usize::from_str_radix(parts[0].trim_start_matches("0x"), 16)
            .or_else(|_| usize::from_str(parts[0]));
        let end = usize::from_str_radix(parts[1].trim_start_matches("0x"), 16)
            .or_else(|_| usize::from_str(parts[1]));

        let (start, end) = match (start, end) {
            (Ok(s), Ok(e)) => (s, e),
            _ => return Err("Range entries must either be in the format '0xFF' or '255'"),
        };

        Ok(ByteRange { start, end })
    }
}

#[derive(Parser, Debug)]
#[clap(name = "hex-thing", about = "A custom hex dump tool", version = "1.0")]
struct Args {
    /// Input file to process
    #[clap(value_name = "FILE")]
    input: PathBuf,

    /// Output to a file instead of standard output
    #[clap(short, long, value_name = "OUTPUT")]
    output: Option<PathBuf>,

    /// Number of bytes per line
    #[clap(short = 'l', long, default_value = "16")]
    bytes_per_line: usize,

    /// Byte range to read (e.g., 0-1000 or 0xff-0x3e7)
    #[clap(short = 'b', long, value_name = "RANGE")]
    byte_range: Option<ByteRange>,

    /// Reverse operation (hexdump to binary)
    #[clap(short = 'r', long, requires = "output")]
    reverse: bool,

    /// Display hex in uppercase (e.g., 0xFF instead of 0xff)
    #[clap(short, long)]
    uppercase: bool,
}

lazy_static! {
    static ref ARGS: Args = Args::parse();
}

const SPLIT_SYMBOL: &str = "┃";

fn get_color(byte: u8) -> Color {
    match byte {
        0 => Color::White,
        9 | 10 | 13 | 32 => Color::Blue,
        32..=126 => Color::Green,
        128..=255 => Color::Yellow,
        _ => Color::Red,
    }
}

fn addr_side(addr: usize, trailing_zeroes: usize) -> String {
    match ARGS.uppercase {
        true => format!("0x{:0width$X}:", addr, width = trailing_zeroes),
        false => format!("0x{:0width$x}:", addr, width = trailing_zeroes),
    }
}

fn hex_side(buff: &Vec<u8>, bytes_read: usize, use_color: bool) -> String {
    let hex_string = hex::encode(buff);

    let spaced_hex = hex_string.as_bytes().chunks(2).map(std::str::from_utf8);
    let spaced_hex = if use_color {
        spaced_hex
            .map(|x| x.unwrap().to_string())
            .collect::<Vec<String>>()
    } else {
        spaced_hex
            .enumerate()
            .map(|(i, x)| x.unwrap().color(get_color(buff[i])).to_string())
            .collect::<Vec<String>>()
    };
    let spaced_hex = spaced_hex.join(" ");

    spaced_hex
}

fn ascii_side(buff: &Vec<u8>, bytes_read: usize, use_color: bool) -> String {
    "ascii work in progress".to_string()
}

fn read_binary_file() -> io::Result<()> {
    let file = File::open(ARGS.input.clone())?;
    let file_size = file.metadata()?.len();
    let trailing_zeroes = (file_size as f64).log(16.0).ceil() as usize;
    let start_point = ARGS.byte_range.clone().unwrap_or_default().start as u64;

    let mut reader = BufReader::new(file);

    reader.seek(io::SeekFrom::Start(start_point))?;

    let buffer_size = ARGS.bytes_per_line; // Size of each buffer read
    let mut buffer = vec![0u8; buffer_size]; // Create a buffer to hold the data

    let mut current_addr = start_point as usize;

    loop {
        // Read into the buffer
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break; // End of file
        }
        if ARGS.byte_range.is_some() && current_addr > ARGS.byte_range.clone().unwrap().end {
            break; // Out of range
        }

        let addr = addr_side(current_addr, trailing_zeroes);
        let hex = hex_side(&buffer, bytes_read, ARGS.output.is_some());
        let ascii = ascii_side(&buffer, bytes_read, ARGS.output.is_some());

        if ARGS.output.is_none() {
            println!(
                "{} {} {} {} {}",
                addr, SPLIT_SYMBOL, hex, SPLIT_SYMBOL, ascii
            )
        } else {
            unimplemented!("No writing to file yet");
        }

        current_addr += bytes_read;
    }

    Ok(())
}

fn reverse_operation() -> io::Result<()> {
    let ifile = File::open(ARGS.input.clone())?;
    let reader = BufReader::new(ifile);

    let mut out_hex: Vec<u8> = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        let parts: Vec<&str> = line.trim().split(SPLIT_SYMBOL).collect();

        let hex_str = match parts.len() {
            1 => parts[0],
            2 | 3 => parts[1],
            _ => {
                eprintln!(
                    "Error: Unrecognized input format for reverse operation on line {}",
                    index + 1
                );
                std::process::exit(1);
            }
        };
        let hex_str = hex_str.replace(" ", "");

        let mut hex = match hex::decode(hex_str.clone()) {
            Ok(bin) => bin,
            Err(_) => {
                eprintln!(
                    "Error: Unable to decode hex \"{}\" on line {} ",
                    hex_str,
                    index + 1
                );
                std::process::exit(1);
            }
        };

        out_hex.append(&mut hex);
    }

    let ofile = File::create(ARGS.output.clone().expect("No output argument found"))?;
    let mut writer = BufWriter::new(ofile);

    writer.write_all(&out_hex)?;
    writer.flush()?;

    Ok(())
}

fn main() -> io::Result<()> {
    if ARGS.reverse && ARGS.output.is_none() {
        eprintln!("Error: -r (reverse) option requires -o (output) to be specified.");
        std::process::exit(1);
    }

    if ARGS.reverse {
        println!("reversing!");
        reverse_operation()?;
    } else {
        read_binary_file()?;
    }

    Ok(())
}
