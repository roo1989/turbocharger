#[allow(unused_imports)]
use wasm_bindgen::prelude::*;

use serde::{Deserialize, Serialize};
use turbocharger::{backend, server_only};
#[cfg(not(target_arch = "wasm32"))]
use turbosql::{select, Turbosql};

#[wasm_bindgen(getter_with_clone, inspectable)]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Turbosql))]
pub struct Person {
 pub rowid: Option<i64>,
 pub name: Option<String>,
}

#[wasm_bindgen]
impl Person {
 #[wasm_bindgen(constructor)]
 pub fn new() -> Person {
  Person::default()
 }
}

#[backend]
pub async fn insert_person(p: Person) -> Result<i64, turbosql::Error> {
 p.insert() // returns rowid
}

#[backend]
// async fn get_person(rowid: i64) -> Result<Person, Box<dyn std::error::Error>> {
async fn get_person(rowid: i64) -> Result<Person, turbosql::Error> {
 select!(Person "WHERE rowid = ?", rowid)
}

#[server_only]
#[tokio::main]
async fn main() {
 eprintln!("Serving on http://127.0.0.1:8080");
 warp::serve(turbocharger::warp_routes()).run(([127, 0, 0, 1], 8080)).await;
}
