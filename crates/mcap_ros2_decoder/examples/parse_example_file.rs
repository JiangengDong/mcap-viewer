#![warn(clippy::pedantic)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::Parser;
use mcap::records::Record;
use rayon::prelude::*;

#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long)]
    path: PathBuf,
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

fn main() {
    let cli = Cli::parse();
    let file_paths = list_all_mcap_files(&cli.path);

    let mut message_table = std::collections::HashMap::new();
    let instant = std::time::Instant::now();

    for file_path in file_paths {
        println!("Parsing {file_path:?}");

        let fd = std::fs::File::open(file_path).unwrap();
        let mapped = unsafe { memmap::Mmap::map(&fd).unwrap() };

        if let Ok(Some(summary)) = mcap::Summary::read(&mapped) {
            for (_, channel) in summary.channels {
                let schema = channel.schema.as_ref().unwrap();
                let schema_name = &schema.name;
                mcap_ros2_decoder::schema::parse(schema_name, &schema.data, &mut message_table)
                    .unwrap();
            }
        } else {
            mcap::read::ChunkFlattener::new_with_options(
                &mapped,
                mcap::read::Options::IgnoreEndMagic.into(),
            )
            .unwrap()
            .map_while(std::result::Result::ok)
            .for_each(|r| {
                if let Record::Schema { header, data } = r {
                    let schema_name = &header.name;
                    mcap_ros2_decoder::schema::parse(schema_name, &data, &mut message_table)
                        .unwrap();
                }
            });
        }

        let message_table_arc = Arc::new(message_table);
        let stream = mcap::MessageStream::new_with_options(
            &mapped,
            mcap::read::Options::IgnoreEndMagic.into(),
        )
        .unwrap()
        .map_while(std::result::Result::ok);
        stream.par_bridge().for_each_init(
            || message_table_arc.clone(),
            |message_table, message| {
                let schema = message.channel.schema.as_ref().unwrap();
                let schema_name = &schema.name;
                let schema = mcap_ros2_decoder::schema::get(schema_name, message_table)
                    .unwrap()
                    .unwrap();
                let mut visitor = mcap_decoder::test_visitor::NoopVisitor;
                mcap_ros2_decoder::decode::decode(&schema, &message.data, &mut visitor).unwrap();
            },
        );

        message_table = Arc::try_unwrap(message_table_arc).unwrap();
    }
    println!("Take {:?} to parse all the mcaps.", instant.elapsed());
}
