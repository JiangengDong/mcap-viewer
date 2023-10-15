#![warn(clippy::pedantic)]

use std::path::{Path, PathBuf};

use clap::Parser;
use mcap::records::Record;
use mcap_decoder::Decoder as _;
use mcap_ros2_decoder::Decoder;
use rayon::prelude::*;

#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long)]
    path: PathBuf,
    #[clap(short, long)]
    use_multi_threads: bool,
}

fn list_all_mcap_files(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        vec![path.to_path_buf()]
    } else if path.is_dir() {
        let mut result = Vec::new();
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                result.extend(list_all_mcap_files(&path));
            } else if path.extension().unwrap() == "mcap" {
                result.push(path);
            }
        }
        result
    } else {
        panic!("Invalid path: {path:?}");
    }
}

fn parse_all_schemas(mapped: &memmap::Mmap, decoder: &Decoder) {
    if let Ok(Some(summary)) = mcap::Summary::read(mapped) {
        for (_, channel) in summary.channels {
            let schema = channel.schema.as_ref().unwrap();
            decoder.parse_schema(&schema.name, &schema.data).unwrap();
        }
    } else {
        let stream = mcap::read::ChunkFlattener::new_with_options(
            mapped,
            mcap::read::Options::IgnoreEndMagic.into(),
        )
        .unwrap();
        stream.map_while(std::result::Result::ok).for_each(|r| {
            if let Record::Schema { header, data } = r {
                decoder.parse_schema(&header.name, &data).unwrap();
            }
        });
    }
}

fn map_file(file_path: PathBuf) -> memmap::Mmap {
    let fd = std::fs::File::open(file_path).unwrap();

    unsafe { memmap::Mmap::map(&fd).unwrap() }
}

fn decode_multi_thread(mapped: &memmap::Mmap, decoder: &Decoder) -> usize {
    let stream =
        mcap::MessageStream::new_with_options(mapped, mcap::read::Options::IgnoreEndMagic.into())
            .unwrap()
            .map_while(std::result::Result::ok);
    stream
        .par_bridge()
        .map_init(
            || decoder.clone(),
            |decoder, message| {
                let schema = message.channel.schema.as_ref().unwrap();
                let mut visitor = mcap_decoder::test_visitor::NoopVisitor;
                decoder
                    .decode(&schema.name, &schema.data, &message.data, &mut visitor)
                    .unwrap();
                1
            },
        )
        .sum()
}

fn decode_single_thread(
    mapped: &memmap::Mmap,
    decoder: &Decoder,
    storage: &mut mcap_viewer_storage::DataStorage,
) {
    let stream =
        mcap::MessageStream::new_with_options(mapped, mcap::read::Options::IgnoreEndMagic.into())
            .unwrap()
            .map_while(std::result::Result::ok);
    stream.for_each(|message| {
        let channel = message.channel;
        let schema = channel.schema.as_ref().unwrap();
        let mut visitor = storage.insert(&channel.topic, message.publish_time);
        decoder
            .decode(&schema.name, &schema.data, &message.data, &mut visitor)
            .unwrap();
    });
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    env_logger::init();

    let cli = Cli::parse();
    if cli.use_multi_threads {
        log::debug!("Use multiple threads.");
    } else {
        log::debug!("Use single threads.");
    }

    let file_paths = list_all_mcap_files(&cli.path);
    let file_count = file_paths.len();

    let decoder = Decoder::new();
    let mut storage = mcap_viewer_storage::DataStorage::new();
    let instant = std::time::Instant::now();

    for file_path in file_paths {
        log::trace!("Parsing {file_path:?}");

        let mapped = map_file(file_path);
        parse_all_schemas(&mapped, &decoder);

        if cli.use_multi_threads {
            // msg_count += decode_multi_thread(&mapped, &decoder);
            unimplemented!();
        } else {
            decode_single_thread(&mapped, &decoder, &mut storage);
        }
    }

    let elapsed_time = instant.elapsed();
    log::info!("Take {elapsed_time:?} to parse {file_count} mcap files.");
}
