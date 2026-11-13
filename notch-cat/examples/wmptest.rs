#[path = "../src/wmp.rs"]
mod wmp;
fn main() {
    println!("running={}", wmp::running());
    let (o, _tx) = wmp::spawn(|s| println!("{s}"));
    for _ in 0..8 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        println!("{:?}", o.lock().unwrap());
    }
}
