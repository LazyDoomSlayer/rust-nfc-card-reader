mod other;
mod write_read;

fn main() {
    println!("Hello, world!");
    let _response_01 = write_read::write_read_nfc();
    let _response_02 = other::read_nfc();
}
