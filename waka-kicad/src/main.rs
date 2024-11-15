use waka_kicad::FindProcess;
// use kicad::{KiCad, KiCadConnectionConfig};
use sysinfo::System;

// fn main() -> Result<(), anyhow::Error> {
fn main() -> Result<(), anyhow::Error> {
  // TODO: blocked by the GitLab issue i submitted
  // let k = KiCad::new(KiCadConnectionConfig {
  //   client_name: String::from("waka-kicad"),
  //   ..Default::default()
  // }).expect("KiCAD not running!");
  // println!("Connected to KiCAD {}", k.get_version().unwrap());
  let mut sys = System::new_all();
  sys.refresh_all();
  // for (pid, process) in sys.processes() {
  //   println!("[{pid}] {:?} {:?}", process.name(), process.exe());
  // }
  // TODO: inaccurate? vvv
  let kicads = sys.processes_by_name("kicad".as_ref());
  let schematic_editors = sys.processes_by_name("eeschema".as_ref());
  let pcb_editors = sys.processes_by_name("pcbnew".as_ref());
  println!("{} instances of KiCad open", kicads.count());
  println!("{} schematic editors open", schematic_editors.count());
  println!("{} pcb editors open", pcb_editors.count());
  Ok(())
}
