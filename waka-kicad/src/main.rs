// use waka_kicad::FindProcess;
// only works on KiCAD nightly
use kicad::{DocumentType, KiCad, KiCadConnectionConfig};
use sysinfo::System;

fn main() -> Result<(), anyhow::Error> {
  // TODO: wait instead of expect
  let k = KiCad::new(KiCadConnectionConfig {
    client_name: String::from("waka-kicad"),
    ..Default::default()
  }).expect("KiCAD not running!");
  println!("Connected to KiCAD {}", k.get_version().unwrap());
  let mut sys = System::new_all();
  sys.refresh_all();
  // TODO: inaccurate? vvv
  // let kicads = sys.processes_by_name("kicad".as_ref());
  // let schematic_editors = sys.processes_by_name("eeschema".as_ref());
  // let pcb_editors = sys.processes_by_name("pcbnew".as_ref());
  // println!("{} instances of KiCad open", kicads.count());
  // println!("{} schematic editors open", schematic_editors.count());
  // println!("{} pcb editors open", pcb_editors.count());
  if let Ok(schematics) = k.get_open_documents(DocumentType::DOCTYPE_SCHEMATIC) {
    println!("Found {} open schematic(s)", schematics.len());
  }
  if let Ok(pcbs) = k.get_open_documents(DocumentType::DOCTYPE_PCB) {
    println!("Found {} open PCB(s)", pcbs.len());
  }
  if let Ok(board) = k.get_open_board() {
    println!("Found open board: {:?}", board);
  }
  Ok(())
}
