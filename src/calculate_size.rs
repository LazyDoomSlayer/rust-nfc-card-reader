use pcsc::*;
use std::error::Error;

/// Reads and calculates the total writable space of an NFC tag/card.
pub fn get_nfc_tag_capacity() -> Result<u32, Box<dyn Error>> {
    let ctx = Context::establish(Scope::User)?;
    println!("PC/SC context established.");

    let mut readers_buf = [0; 2048];
    let mut readers = ctx.list_readers(&mut readers_buf)?;
    let reader = match readers.next() {
        Some(reader) => reader,
        None => {
            println!("No readers are connected.");
            return Ok(0);
        }
    };
    println!("Using reader: {:?}", reader);

    let mut card = ctx.connect(reader, ShareMode::Shared, Protocols::ANY)?;
    println!("Card connected.");

    let tx = card.transaction()?;
    println!("Transaction started.");

    let mut total_space = 0;

    for page in 0..=255 {
        let read_command = [
            0xFF, // Class
            0xB0, // Instruction (Read Binary)
            0x00, // P1 (Memory address high byte)
            page, // P2 (Memory address low byte, page number)
            0x04, // Le (Expected data length: 4 bytes per page)
        ];

        let mut response_buf = [0; 256];
        let response = tx.transmit(&read_command, &mut response_buf);

        match response {
            Ok(data) => {
                // Add the size of each page to the total space
                total_space += data.len() as u32;

                // Stop if terminator is detected
                if data.contains(&0xFE) {
                    println!("Detected terminator (0xFE) at page {}.", page);
                    break;
                }
            }
            Err(err) => {
                eprintln!("Failed to read page {}: {}", page, err);
                break;
            }
        }
    }

    match tx.end(Disposition::LeaveCard) {
        Ok(_) => println!("Transaction ended successfully."),
        Err((_, err)) => eprintln!("Failed to end transaction: {}", err),
    }

    match card.disconnect(Disposition::ResetCard) {
        Ok(_) => println!("Card disconnected successfully."),
        Err((_, err)) => eprintln!("Failed to disconnect card: {}", err),
    }

    println!("Total NFC tag capacity: {} bytes", total_space);
    Ok(total_space)
}
