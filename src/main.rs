use pcsc::*;
fn start_reading() -> Result<(), Box<dyn std::error::Error>> {
    print!("Starting reading... ");

    let ctx = Context::establish(Scope::User)?;
    println!("PC/SC context established.");

    let mut readers_buf = [0; 2048];
    let mut readers = ctx.list_readers(&mut readers_buf)?;
    let reader = match readers.next() {
        Some(reader) => reader,
        None => {
            println!("No readers are connected.");
            return Ok(());
        }
    };
    println!("Using reader: {:?}", reader);

    let mut card = ctx.connect(reader, ShareMode::Shared, Protocols::ANY)?;
    println!("Card connected.");

    let mut tx: Transaction = card.transaction()?;
    println!("Transaction started.");

    let data = read_entire_card(&mut tx, 1024, 16)?;

    println!("Collected data: {:?}", data);
    Ok(())
}

fn read_entire_card(
    tx: &mut Transaction,
    total_memory_size: usize,
    block_size: usize,
) -> Result<Vec<u8>, Error> {
    println!("Reading entire NFC card memory...");

    let mut data = Vec::new();
    let total_blocks = (total_memory_size + block_size - 1) / block_size;

    for block in 0..total_blocks {
        let apdu_command = [
            0x00,                 // CLA
            0xB0,                 // INS: Read binary
            (block >> 8) as u8,   // P1: High byte of block address
            (block & 0xFF) as u8, // P2: Low byte of block address
            block_size as u8,     // Le: Number of bytes to read
        ];

        let mut response_buffer = [0; MAX_BUFFER_SIZE];
        let response = tx.transmit(&apdu_command, &mut response_buffer)?;

        if response.len() >= 2 && &response[response.len() - 2..] == [0x90, 0x00] {
            data.extend_from_slice(&response[..response.len() - 2]);
            println!("Block {}: {:?}", block, &response[..response.len() - 2]);
        } else {
            println!("Failed to read block {}: Response: {:?}", block, response);
            break;
        }
    }

    println!(
        "Finished reading card memory. Total bytes read: {}",
        data.len()
    );
    Ok(data)
}

fn main() {
    println!("Hello, world!");
    start_reading().expect("TODO: panic message");
}
