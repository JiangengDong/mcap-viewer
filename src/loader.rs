use std::path::{Path, PathBuf};

use mcap::records::Record;
use mcap_decoder::Decoder as DecoderTrait;
use mcap_ros2_decoder::Decoder;
use mcap_viewer_storage::DataStorage;

pub fn list_all_mcap_files(path: &Path) -> Vec<PathBuf> {
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

pub fn parse_all_schemas(bytes: &[u8], decoder: &Decoder) {
    log::info!("Start to parse the schema.");
    if let Ok(Some(summary)) = mcap::Summary::read(bytes) {
        for (_, channel) in summary.channels {
            let schema = channel.schema.as_ref().unwrap();
            decoder.parse_schema(&schema.name, &schema.data).unwrap();
        }
    } else {
        let stream = mcap::read::ChunkFlattener::new_with_options(
            bytes,
            mcap::read::Options::IgnoreEndMagic.into(),
        )
        .unwrap();
        stream.map_while(std::result::Result::ok).for_each(|r| {
            if let Record::Schema { header, data } = r {
                decoder.parse_schema(&header.name, &data).unwrap();
            }
        });
    }
    log::info!("Finish parsing the schema.");
}

pub fn decode_single_thread(bytes: &[u8], decoder: &Decoder, storage: &mut DataStorage) {
    log::info!("Start to decode the mcap file.");
    let stream =
        mcap::MessageStream::new_with_options(bytes, mcap::read::Options::IgnoreEndMagic.into())
            .unwrap()
            .map_while(std::result::Result::ok);
    stream.for_each(|message| {
        let channel = message.channel;
        let schema = channel.schema.as_ref().unwrap();
        let mut visitor = storage.new_visitor(&channel.topic, message.publish_time);
        decoder
            .decode(&schema.name, &schema.data, &message.data, &mut visitor)
            .unwrap();
    });
    log::info!("Finish decoding the mcap file.");
}
