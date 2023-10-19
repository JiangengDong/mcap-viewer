use egui_plot::PlotPoint;
use mcap_viewer_storage::DataStorage;
use nohash_hasher::IntMap;

#[derive(Default)]
pub struct PlotPointStorage {
    generation: u32,
    cache: IntMap<u64, (u32, Vec<PlotPoint>)>,
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
    pub fn get(&mut self, key: (&str, &str)) -> Vec<PlotPoint> {
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
                    .map(Vec::<PlotPoint>::from)
                    .unwrap_or_default();
                entry.insert((self.generation, value.clone()));
                value
            }
        }
    }

    pub fn inner(&self) -> &DataStorage {
        &self.storage
    }
}
