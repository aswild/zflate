use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, ValueEnum};
use flate2::{
    bufread::{DeflateDecoder, DeflateEncoder, GzDecoder, GzEncoder, ZlibDecoder, ZlibEncoder},
    Compression,
};

/// Compress or decompress zlib-formatted data streams
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

    /// Output filename. When no FILE, write to standard output
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    #[command(flatten)]
    comp_level_args: CompressionLevelArgs,

    /// Input file(s). When no FILE, read standard input
    #[arg(value_name = "FILE")]
    files: Option<Vec<PathBuf>>,
}

#[derive(Debug, Clone, Copy, clap::Args)]
#[group(required = false, multiple = false)]
struct CompressionLevelArgs {
    /// Compression level: use args -1 (fastest) through -9 (best)
    ///
    /// The default compression level is 6
    #[arg(short = '1')]
    level1: bool,

    #[arg(short = '2', hide = true)]
    level2: bool,
    #[arg(short = '3', hide = true)]
    level3: bool,
    #[arg(short = '4', hide = true)]
    level4: bool,
    #[arg(short = '5', hide = true)]
    level5: bool,
    #[arg(short = '6', hide = true)]
    level6: bool,
    #[arg(short = '7', hide = true)]
    level7: bool,
    #[arg(short = '8', hide = true)]
    level8: bool,
    #[arg(short = '9', hide = true)]
    level9: bool,
}

impl CompressionLevelArgs {
    fn level(&self) -> u32 {
        if self.level1 {
            1
        } else if self.level2 {
            2
        } else if self.level3 {
            3
        } else if self.level4 {
            4
        } else if self.level5 {
            5
        } else if self.level6 {
            6
        } else if self.level7 {
            7
        } else if self.level8 {
            8
        } else if self.level9 {
            9
        } else {
            Compression::default().level()
        }
    }

    fn compression(&self) -> Compression {
        Compression::new(self.level())
    }
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

    let comp_level = args.comp_level_args.compression();
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
