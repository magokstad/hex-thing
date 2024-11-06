use std::{
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Read, Seek, Write},
    path::PathBuf,
};

use byte_range::ByteRange;
use clap::Parser;
use colored::{Color, Colorize};
use lazy_static::lazy_static;
use util::ApplyIf;

mod byte_range;
mod util;

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

    /// Skip the N first bytes of the file
    #[clap(short, long, value_name = "N")]
    skip: Option<usize>,

    /// Only read N bytes from input
    #[clap(short = 'n', long, value_name = "N")]
    length: Option<usize>,

    /// Byte range to read (e.g., 0-1000 or 0xff-0x3e7)
    #[clap(
        long,
        value_name = "RANGE",
        conflicts_with_all = ["skip", "length"],
    )]
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
    static ref USE_COLOR: bool = ARGS.output.is_none();
    static ref RAW_SPLIT_SYMBOL: &'static str = "│";
    static ref SPLIT_SYMBOL: String = match *USE_COLOR {
        true => RAW_SPLIT_SYMBOL.color(Color::BrightBlack).to_string(),
        false => RAW_SPLIT_SYMBOL.to_string(),
    };
    static ref START: usize = match ARGS.byte_range.clone() {
        Some(range) => range.start,
        None => match ARGS.skip {
            Some(num) => num,
            None => 0,
        },
    };
    static ref MAX_COUNT: Option<usize> = match ARGS.byte_range.clone() {
        Some(range) => Some(range.end - range.start),
        None => match ARGS.length {
            Some(num) => Some(num),
            None => None,
        },
    };
}

fn get_color(byte: u8) -> Color {
    match byte {
        0 => Color::BrightBlack,
        9 | 10 | 13 | 32 => Color::Cyan,
        32..=126 => Color::Green,
        128..=255 => Color::Yellow,
        _ => Color::BrightRed,
    }
}

fn get_ascii(byte: u8) -> String {
    match byte {
        0 => "•".to_string(),
        9 => "⇥".to_string(),
        10 => "␊".to_string(),
        13 => "␍".to_string(),
        32 => "␣".to_string(),
        32..=126 => (byte as char).to_string(),
        128..=255 => "×".to_string(),
        _ => "▴".to_string(),
    }
}

fn addr_line(addr: usize, trailing_zeroes: usize, use_color: bool) -> String {
    match ARGS.uppercase {
        true => format!("0x{:0width$X}", addr, width = trailing_zeroes),
        false => format!("0x{:0width$x}", addr, width = trailing_zeroes),
    }
    .apply_if(use_color, |x| x.color(Color::BrightBlack).to_string())
}

fn hex_line(buff: &[u8], bytes_read: usize, use_color: bool) -> String {
    hex::encode(buff)
        .as_bytes()
        .chunks(2)
        .take(bytes_read)
        .map(std::str::from_utf8)
        .enumerate()
        .map(|(index, hex)| {
            hex.unwrap()
                .to_string()
                .apply_if(ARGS.uppercase, |hex_string| hex_string.to_uppercase())
                .apply_if(use_color, |hex_string| {
                    hex_string.color(get_color(buff[index])).to_string()
                })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn ascii_line(buff: &[u8], bytes_read: usize, use_color: bool) -> String {
    buff.into_iter()
        .take(bytes_read)
        .map(|&byte| {
            get_ascii(byte).apply_if(use_color, |byte_string| {
                byte_string.color(get_color(byte)).to_string()
            })
        })
        .collect()
}

fn read_binary_file() -> io::Result<()> {
    let ifile = File::open(ARGS.input.clone())?;
    let ifile_size = ifile.metadata()?.len();

    let trailing_zeroes = (ifile_size as f64).log(16.0).ceil() as usize;

    let mut reader = BufReader::new(ifile);
    reader.seek(io::SeekFrom::Start(*START as u64))?;

    let buffer_size = ARGS.bytes_per_line;
    let mut buffer = vec![0u8; buffer_size];

    let mut current_addr = *START;
    let mut total_bytes_read = 0;

    let mut writer = match ARGS.output.clone() {
        Some(of_name) => Some(BufWriter::new(File::create_new(of_name)?)),
        None => None,
    };

    loop {
        let bytes_read = reader.read(&mut buffer)?;

        if bytes_read == 0 {
            // End of file
            break;
        }

        if MAX_COUNT.is_some() && total_bytes_read >= MAX_COUNT.unwrap() {
            // Out of range
            break;
        }

        let bytes_read =
            if MAX_COUNT.is_some() && total_bytes_read + bytes_read >= MAX_COUNT.unwrap() {
                MAX_COUNT.unwrap() - total_bytes_read
            } else {
                bytes_read
            };

        let addr = addr_line(current_addr, trailing_zeroes, *USE_COLOR);
        let hex = hex_line(&buffer, bytes_read, *USE_COLOR);
        let ascii = ascii_line(&buffer, bytes_read, *USE_COLOR);

        let extra_space = " ".repeat((ARGS.bytes_per_line - bytes_read) * 3);

        let output = format!(
            " {} {} {}{} {} {}\n",
            addr, *SPLIT_SYMBOL, hex, extra_space, *SPLIT_SYMBOL, ascii
        );
        match &mut writer {
            Some(w) => w.write_all(output.as_bytes())?,
            None => {
                print!("{output}");
            }
        };

        current_addr += bytes_read;
        total_bytes_read += bytes_read;
    }

    if let Some(w) = &mut writer {
        w.flush()?;
    }

    Ok(())
}

fn reverse_operation() -> io::Result<()> {
    let ifile = File::open(ARGS.input.clone())?;
    let reader = BufReader::new(ifile);

    let mut out_hex: Vec<u8> = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        let parts: Vec<&str> = line.trim().split(*RAW_SPLIT_SYMBOL).collect();

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
    match ARGS.reverse {
        true => reverse_operation()?,
        false => read_binary_file()?,
    };

    Ok(())
}
