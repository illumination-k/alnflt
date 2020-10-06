use structopt::clap::arg_enum;
use structopt::{clap, StructOpt};

use anyhow::Result;
use atty;

use bio_types::strand::ReqStrand;
use rust_htslib::{bam, bam::Read};

#[derive(Debug, StructOpt)]
#[structopt(name = "alnflt")]
#[structopt(long_version(option_env!("LONG_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))))]
#[structopt(setting(clap::AppSettings::ColoredHelp))]
pub struct Opt {
    #[structopt(name = "INPUT")]
    pub input: Option<String>,
    #[structopt(short = "t", long = "threads", default_value = "1", value_name = "INT")]
    pub threads: usize,
    #[structopt(short = "o", long = "out")]
    pub output: Option<String>,
    #[structopt(long = "filterStrand", possible_values(&Strand::variants()))]
    pub filter_strand: Option<Strand>,
    #[structopt(long = "minMappingQuality", value_name = "INT")]
    pub min_mapping_quality: Option<u8>,
    #[structopt(long = "minInsertSize", value_name = "INT")]
    pub min_insertsize: Option<i64>,
    #[structopt(long = "maxInsertSize", value_name = "INT")]
    pub max_insertsize: Option<i64>,
}

arg_enum! {
    #[derive(Debug)]
    pub enum Strand {
        Forward,
        Reverse,
    }
}

fn is_stdin(input: Option<&String>) -> bool {
    let is_request = match input {
        Some(i) if i == "-" => true,
        _ => false,
    };

    let is_pipe = !atty::is(atty::Stream::Stdin);

    is_request || is_pipe
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    if opt.input.is_none() && !is_stdin(opt.input.as_ref()) {
        Opt::clap().print_help()?;
        std::process::exit(1);
    }

    let mut bam = if is_stdin(opt.input.as_ref()) {
        bam::Reader::from_stdin()?
    } else {
        let input = std::path::PathBuf::from(opt.input.unwrap());
        bam::Reader::from_path(input)?
    };

    // set reader
    bam.set_threads(opt.threads)?;

    // set writer
    let header = bam::Header::from_template(&bam.header());

    let mut writer = match opt.output {
        Some(output) => {
            let out_path = std::path::PathBuf::from(output);
            bam::Writer::from_path(out_path, &header, bam::Format::BAM)?
        }
        None => bam::Writer::from_stdout(&header, bam::Format::BAM)?,
    };
    writer.set_threads(opt.threads)?;

    for (i, _r) in bam.records().enumerate() {
        if i == 10 {
            break;
        }
        let mut r = _r?;

        // mapping quality
        match &opt.min_mapping_quality {
            Some(min_mapping_quality) if &r.mapq() < min_mapping_quality => {
                continue;
            }
            _ => (),
        }

        // strand
        match &opt.filter_strand {
            Some(strand) => match *strand {
                Strand::Forward => match &r.strand() {
                    ReqStrand::Forward => (),
                    ReqStrand::Reverse => {
                        continue;
                    }
                },
                Strand::Reverse => match &r.strand() {
                    ReqStrand::Forward => {
                        continue;
                    }
                    ReqStrand::Reverse => (),
                },
            },
            None => (),
        }

        // insertsize
        let insertsize = r.insert_size().abs();

        match &opt.min_insertsize {
            Some(min_insertsize) if insertsize < *min_insertsize => {
                continue;
            }
            _ => (),
        };

        match &opt.max_insertsize {
            Some(max_insertsize) if insertsize > *max_insertsize => {
                continue;
            }
            _ => (),
        }

        writer.write(&r)?;
    }

    Ok(())
}
