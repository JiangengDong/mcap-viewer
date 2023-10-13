#![warn(clippy::pedantic)]

use std::sync::Arc;

use mcap::records::Record;
use rayon::prelude::*;

fn main() {
    let instant = std::time::Instant::now();
    let test_file = env!("CARGO_MANIFEST_DIR").to_string() + "/examples/example2.mcap";
    let fd = std::fs::File::open(test_file).unwrap();
    let mapped = unsafe { memmap::Mmap::map(&fd).unwrap() };

    let mut message_table = std::collections::HashMap::new();
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
                mcap_ros2_decoder::schema::parse(schema_name, &data, &mut message_table).unwrap();
            }
        });
    }

    let message_table = Arc::new(message_table);
    let stream =
        mcap::MessageStream::new_with_options(&mapped, mcap::read::Options::IgnoreEndMagic.into())
            .unwrap()
            .map_while(std::result::Result::ok);
    stream.par_bridge().for_each_init(
        || message_table.clone(),
        |message_table, message| {
            let schema = message.channel.schema.as_ref().unwrap();
            let schema_name = &schema.name;
            let schema = mcap_ros2_decoder::schema::get(schema_name, message_table)
                .unwrap()
                .unwrap();
            let mut visitor = mcap_decoder::test_visitor::NoopVisitor;
            mcap_ros2_decoder::decoder::decode(&schema, &message.data, &mut visitor).unwrap();
        },
    );
    println!("Take {:?} to parse the mcap.", instant.elapsed());
}
