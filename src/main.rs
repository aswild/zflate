use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, ValueEnum};
use flate2::{
    bufread::{DeflateDecoder, DeflateEncoder, GzDecoder, GzEncoder, ZlibDecoder, ZlibEncoder},
    Compression,
};

/// Compress or decompress zlib, gzip, or raw DEFLATE data streams
#[derive(Debug, Parser)]
#[command(version)]
struct Args {
    /// Decompression
    #[arg(short, long)]
    decompress: bool,

    /// Header format: zlib, deflate, or gzip
    ///
    /// Valid aliases include z, d, g, and gz
    #[arg(short, long, value_enum, default_value_t, hide_possible_values = true)]
    mode: Mode,

    /// Compression level: from 1 (fastest) to 9 (best)
    #[arg(
        short, long, value_name = "LEVEL",
        default_value_t = Compression::default().level(),
        value_parser = clap::value_parser!(u32).range(1..=9),
        conflicts_with = "decompress",
    )]
    compression_level: u32,

    /// Output filename. When no FILE, write to standard output
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Input file(s). When no FILE, read standard input
    #[arg(value_name = "FILE")]
    files: Option<Vec<PathBuf>>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Mode {
    #[value(alias = "z")]
    Zlib,
    #[value(alias = "d")]
    Deflate,
    #[value(aliases = ["g", "gz"])]
    Gzip,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Zlib
    }
}

impl Mode {
    fn compress<R, W>(self, level: Compression, input: &mut R, output: &mut W) -> io::Result<u64>
    where
        R: BufRead + ?Sized,
        W: Write + ?Sized,
    {
        match self {
            Mode::Zlib => io::copy(&mut ZlibEncoder::new(input, level), output),
            Mode::Deflate => io::copy(&mut DeflateEncoder::new(input, level), output),
            Mode::Gzip => io::copy(&mut GzEncoder::new(input, level), output),
        }
    }

    fn decompress<R, W>(self, input: &mut R, output: &mut W) -> io::Result<u64>
    where
        R: BufRead + ?Sized,
        W: Write + ?Sized,
    {
        match self {
            Mode::Zlib => io::copy(&mut ZlibDecoder::new(input), output),
            Mode::Deflate => io::copy(&mut DeflateDecoder::new(input), output),
            Mode::Gzip => io::copy(&mut GzDecoder::new(input), output),
        }
    }
}

fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut output: Box<dyn Write> = match &args.output {
        Some(path) => Box::new(BufWriter::new(
            File::create(path).context("failed to open output file")?,
        )),
        None => Box::new(io::stdout()),
    };

    let comp_level = Compression::new(args.compression_level);
    let mut transcode = |input: &mut dyn BufRead| -> io::Result<u64> {
        if args.decompress {
            args.mode.decompress(input, &mut output)
        } else {
            args.mode.compress(comp_level, input, &mut output)
        }
    };

    if let Some(files) = args.files {
        for path in files {
            let mut file = BufReader::new(
                File::open(&path)
                    .with_context(|| format!("failed to open input file '{}'", path.display()))?,
            );
            transcode(&mut file)?;
        }
    } else {
        transcode(&mut io::stdin().lock())?;
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }
}
