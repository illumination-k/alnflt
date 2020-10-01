use structopt::{clap, StructOpt};
use anyhow::Result;
use atty;

use rust_htslib::{bam, bam::Read};

#[derive(Debug, StructOpt)]
#[structopt(name = "alnflt")]
#[structopt(long_version(option_env!("LONG_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))))]
#[structopt(setting(clap::AppSettings::ColoredHelp))]
pub struct Opt {
    #[structopt(name = "INPUT")]
    pub input: Option<String>,
    #[structopt(short = "t", long = "threds", default_value="1", value_name="INT")]
    pub threads: usize,
    #[structopt(short = "o", long = "out")]
    pub output: Option<String>,
    #[structopt(long = "minInsertSize", value_name="INT")]
    pub min_insertsize: Option<i64>,
}


fn is_stdin(input: Option<&String>) -> bool {
    let is_request = match input {
        Some(i) if i == "-" => true,
        None => true,
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
    // let mut bam = bam::Reader::from_path(input)?;
    bam.set_threads(opt.threads)?;

    // set writer
    let header = bam::Header::from_template(&bam.header());
    // let output = std::path::PathBuf::from(opt.output.unwrap());
    // let mut writer = bam::Writer::from_path(output, &header, bam::Format::BAM)?;

    let mut writer = match opt.output {
        Some(output) => {
            let out_path = std::path::PathBuf::from(output);
            bam::Writer::from_path(out_path, &header, bam::Format::BAM)?
        },
        None => bam::Writer::from_stdout(&header, bam::Format::BAM)?
    };
    writer.set_threads(opt.threads)?;

    for (i, _r) in bam.records().enumerate() {
        if i == 10 { break; }
        let r = _r?;
        let insertsize = r.insert_size();
        
        match opt.min_insertsize {
            Some(min_insertsize) if insertsize < min_insertsize => {continue;},
            _ => (),
        };

        writer.write(&r)?;
    }

    Ok(())
}
