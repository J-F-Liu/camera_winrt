use std::collections::BTreeMap;

pub struct IdManager {
    current_id: usize,
    ids: BTreeMap<usize, String>,
}

impl IdManager {
    pub fn new() -> IdManager {
        IdManager {
            current_id: 0,
            ids: BTreeMap::new(),
        }
    }

    pub fn add<T: Into<String>>(&mut self, name: T) -> usize {
        self.current_id += 1;
        self.ids.insert(self.current_id, name.into());
        self.current_id
    }

    pub fn get(&self, id: usize) -> &str {
        &self.ids[&id]
    }
}
