use egui_plot::PlotPoint;
use mcap_viewer_storage::DataStorage;
use nohash_hasher::IntMap;

#[derive(Hash)]
pub struct Key<'a> {
    pub field: &'a str,
    pub name: &'a str,
    pub time_range: [f64; 2],
    pub num_points: usize,
}

#[derive(Default)]
pub struct PlotPointStorage {
    generation: u32,
    cache: IntMap<u64, (u32, Vec<[f64; 2]>)>,
    storage: DataStorage,
}

impl PlotPointStorage {
    pub fn new(storage: DataStorage) -> Self {
        Self {
            generation: 0,
            cache: IntMap::default(),
            storage,
        }
    }

    /// Must be called once per frame to clear the cache.
    pub fn evice_cache(&mut self) {
        let current_generation = self.generation;
        self.cache.retain(|_key, cached| {
            cached.0 == current_generation // only keep those that were used this frame
        });
        self.generation = self.generation.wrapping_add(1);
    }

    /// Get from cache (if the same key was used last frame)
    /// or recompute and store in the cache.
    pub fn get(&mut self, key: Key) -> Vec<[f64; 2]> {
        let hash = egui::util::hash(key);

        match self.cache.entry(hash) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                let cached = entry.into_mut();
                cached.0 = self.generation;
                cached.1.clone()
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let value = self
                    .storage
                    .get_field(key.0, key.1)
                    .map(Vec::<[f64; 2]>::from)
                    .unwrap_or_default();
                let value = Self::downsample(value, key.time_range, key.num_points);
                entry.insert((self.generation, value.clone()));
                value
            }
        }
    }

    fn downsample(data: Vec<[f64; 2]>, time_range: [f64; 2], num_points: usize) -> Vec<[f64; 2]> {
        if data.is_empty() {
            return data;
        }

        let start_idx = if data.first().unwrap()[0] >= time_range[0] {
            0
        } else {
            data.partition_point(|p| p[0] < time_range[0])
                .saturating_sub(20)
        };

        let end_idx = if data.last().unwrap()[0] <= time_range[1] {
            data.len()
        } else {
            data.partition_point(|p| p[0] < time_range[1])
                .saturating_add(20)
                .min(data.len())
        };

        let slice = &data[start_idx..end_idx];
        let xs = slice.iter().map(|pair| pair[0]).collect::<Vec<_>>();
        let ys = slice.iter().map(|pair| pair[1]).collect::<Vec<_>>();
        let indices = downsample_rs::minmaxlttb_with_x(&xs, &ys, num_points, 30);

        indices.into_iter().map(|index| slice[index]).collect()
    }

    pub fn inner(&self) -> &DataStorage {
        &self.storage
    }
}
