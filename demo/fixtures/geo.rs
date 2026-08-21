pub struct GeoDb {
    name: String,
    hits: u32,
}

impl GeoDb {
    pub fn lookup(&self, query: &str) -> bool {
        self.name.contains(query)
    }
}

fn main() {
    let db = GeoDb {
        name: String::from("feature/geo-db"),
        hits: 12,
    };
    println!("ready {}", db.hits);
}
