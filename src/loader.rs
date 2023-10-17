use std::path::{Path, PathBuf};

use mcap::records::Record;
use mcap_decoder::Decoder as DecoderTrait;
use mcap_ros2_decoder::Decoder;
use mcap_viewer_storage::DataStorage;
use rayon::prelude::*;

fn list_all_mcap_files(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        if path.extension().is_some_and(|ext| ext == "mcap") {
            vec![path.to_path_buf()]
        } else {
            Vec::new()
        }
    } else if path.is_dir() {
        let mut result = Vec::new();
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                result.extend(list_all_mcap_files(&path));
            } else if path.extension().is_some_and(|ext| ext == "mcap") {
                result.push(path);
            }
        }
        result
    } else {
        panic!("Invalid path: {path:?}");
    }
}

struct LoaderMessage {
    paths: Vec<PathBuf>,
    response_tx: crossbeam_channel::Sender<DataStorage>,
    context: egui::Context,
}

pub struct Loader {
    request_tx: crossbeam_channel::Sender<LoaderMessage>,
    response_rx: Option<crossbeam_channel::Receiver<DataStorage>>,
}

impl Loader {
    pub fn new() -> Self {
        let (request_tx, request_rx) = crossbeam_channel::unbounded();
        std::thread::spawn(move || Self::worker(request_rx));
        Self {
            request_tx,
            response_rx: None,
        }
    }

    fn worker(request_rx: crossbeam_channel::Receiver<LoaderMessage>) {
        for LoaderMessage {
            paths,
            response_tx,
            context,
        } in request_rx
        {
            let mut storage = DataStorage::default();
            let decoder = Decoder::default();
            let paths = paths
                .into_iter()
                .flat_map(|path| list_all_mcap_files(&path));
            for path in paths {
                log::info!("Start to load the mcap file: {path:?}");
                let bytes = std::fs::read(&path).unwrap();
                parse_all_schemas(&bytes, &decoder);
                decode_multi_thread(&bytes, &decoder, &mut storage);
                log::info!("Finish loading the mcap file: {path:?}");
            }
            storage.sort_unstable();
            response_tx.send(storage).unwrap();
            context.request_repaint();
        }
    }

    pub fn send(&mut self, paths: Vec<PathBuf>, context: &egui::Context) {
        let (response_tx, response_rx) = crossbeam_channel::bounded(1);
        self.request_tx
            .send(LoaderMessage {
                paths,
                response_tx,
                context: context.clone(),
            })
            .unwrap();
        self.response_rx = Some(response_rx);
    }

    pub fn try_recv(&mut self) -> Option<DataStorage> {
        if let Some(response_rx) = &self.response_rx {
            if let Ok(storage) = response_rx.try_recv() {
                self.response_rx = None;
                return Some(storage);
            }
        }
        None
    }
}

fn parse_all_schemas(bytes: &[u8], decoder: &Decoder) {
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

#[allow(unused)]
fn decode_single_thread(bytes: &[u8], decoder: &Decoder, storage: &mut DataStorage) {
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

#[allow(unused)]
fn decode_multi_thread(bytes: &[u8], decoder: &Decoder, storage: &mut DataStorage) {
    let stream =
        mcap::MessageStream::new_with_options(bytes, mcap::read::Options::IgnoreEndMagic.into())
            .unwrap()
            .map_while(std::result::Result::ok);
    let new_storage = stream
        .par_bridge()
        .fold(
            || (decoder.clone(), DataStorage::new()),
            |(decoder, mut storage), message| {
                let channel = message.channel;
                let schema = channel.schema.as_ref().unwrap();
                let mut visitor = storage.new_visitor(&channel.topic, message.publish_time);
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
