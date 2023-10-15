#![warn(clippy::pedantic)]

use std::path::{Path, PathBuf};

use clap::Parser;
use mcap::{records::Record, Message};
use mcap_decoder::Decoder as _;
use mcap_ros2_decoder::Decoder;
use mcap_viewer_storage::DataStorage;
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

fn decode_multi_thread<S>(stream: S, decoder: &Decoder, storage: &mut DataStorage)
where
    S: Iterator<Item = Message<'static>> + Send,
{
    let new_storage = stream
        .par_bridge()
        .fold(
            || (decoder.clone(), DataStorage::new()),
            |(decoder, mut storage), message| {
                let channel = message.channel;
                let schema = channel.schema.as_ref().unwrap();
                let mut visitor = storage.insert(&channel.topic, message.publish_time);
                decoder
                    .decode(&schema.name, &schema.data, &message.data, &mut visitor)
                    .unwrap();
                (decoder, storage)
            },
        )
        .map(|(_, storage)| storage)
        .reduce_with(|mut storage1, storage2| {
            storage1.merge(storage2);
            storage1
        });
    if let Some(new_storage) = new_storage {
        storage.merge(new_storage);
    }
}

fn decode_single_thread<S>(stream: S, decoder: &Decoder, storage: &mut DataStorage)
where
    S: Iterator<Item = Message<'static>> + Send,
{
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
    let mut storage = DataStorage::new();
    let instant = std::time::Instant::now();

    let mapped_files: Vec<_> = file_paths.into_iter().map(map_file).collect();
    for mapped in &mapped_files {
        parse_all_schemas(mapped, &decoder);
    }
    let stream = mapped_files.iter().flat_map(|mapped| {
        mcap::MessageStream::new_with_options(mapped, mcap::read::Options::IgnoreEndMagic.into())
            .unwrap()
            .map_while(std::result::Result::ok)
    });

    if cli.use_multi_threads {
        decode_multi_thread(stream, &decoder, &mut storage);
    } else {
        decode_single_thread(stream, &decoder, &mut storage);
    }

    let elapsed_time = instant.elapsed();
    log::info!("Take {elapsed_time:?} to parse {file_count} mcap files.");
}
